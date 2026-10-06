//! 两端消息编解码：全部走 serde_json，名称里的引号 / 反斜杠 / 换行不再破坏消息（E-01）。
//! 同时负责解析腕端返回，字段缺失或类型异常时给出可读错误而不是静默丢数据（E-12）。

use serde_json::{json, Value};

use crate::ui::state::EventData;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventRef {
    Id(String),
    Index(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseIssue {
    NotJson,
    MissingData,
    NotArray,
}

impl ParseIssue {
    pub fn message(self) -> &'static str {
        match self {
            ParseIssue::NotJson => "手环端返回不是合法 JSON",
            ParseIssue::MissingData => "手环端返回缺少 data 字段",
            ParseIssue::NotArray => "手环端返回的 data 不是数组",
        }
    }
}

pub const PKG_NAME: &str = "com.yzf.daymatter";
pub const MIN_APP_VERSION_CODE: u32 = 21000;

pub fn add_event(draft: &EventData) -> String {
    json!({
        "type": "addEvent",
        "name": draft.name,
        "date": draft.time,
        "on_index": draft.on_index,
        "IFStaringDay": draft.if_staring_day,
    })
    .to_string()
}

/// 修改消息：优先带稳定 ID，旧版腕端 / 无 ID 事件退回下标（E-11）
pub fn change_event(draft: &EventData, target: &EventRef) -> String {
    let mut payload = json!({
        "type": "changeEvent",
        "name": draft.name,
        "date": draft.time,
        "on_index": draft.on_index,
        "IFStaringDay": draft.if_staring_day,
    });
    match target {
        EventRef::Id(id) => payload["id"] = json!(id),
        EventRef::Index(index) => payload["index"] = json!(index),
    }
    payload.to_string()
}

pub fn delete_event(target: &EventRef) -> String {
    let mut payload = json!({"type": "deleteEvent"});
    match target {
        EventRef::Id(id) => payload["id"] = json!(id),
        EventRef::Index(index) => payload["index"] = json!(index),
    }
    payload.to_string()
}

pub fn get_all_events() -> String {
    json!({"type": "getAllEvent"}).to_string()
}

pub fn add_background(base64_data: &str) -> String {
    json!({"type": "addBG", "bgBase64": base64_data}).to_string()
}

/// 解析腕端事件列表：兼容 `payloadText` 包裹；单条异常只跳过该条并计数
#[derive(Debug, PartialEq)]
pub struct ParsedEvents {
    pub events: Vec<EventData>,
    pub skipped: usize,
    pub primary_id: Option<String>,
    pub revision: Option<u64>,
}

pub fn parse_event_list(payload: &str) -> Result<ParsedEvents, ParseIssue> {
    let outer: Value = serde_json::from_str(payload).map_err(|_| ParseIssue::NotJson)?;
    let wrapped_text = outer
        .get("payloadText")
        .or_else(|| outer.get("payload"))
        .and_then(Value::as_str);
    let inner: Value = match wrapped_text {
        Some(text) => serde_json::from_str(text).map_err(|_| ParseIssue::NotJson)?,
        None => outer,
    };
    let data = inner.get("data").ok_or(ParseIssue::MissingData)?;
    // App `connect.send({data: {data: events, primaryId, revision}})` nests
    // the list under data.data; older envelopes send data as an array directly.
    let (array, metadata) = match data {
        Value::Array(items) => (items, &inner),
        Value::Object(map) => {
            let array = map
                .get("data")
                .or_else(|| map.get("events"))
                .and_then(Value::as_array)
                .ok_or(ParseIssue::NotArray)?;
            let metadata = if map.get("primaryId").is_some() || map.get("revision").is_some() {
                data
            } else {
                &inner
            };
            (array, metadata)
        }
        _ => return Err(ParseIssue::NotArray),
    };

    let mut events = Vec::new();
    let mut skipped = 0;
    for item in array {
        match parse_event(item) {
            Some(event) => events.push(event),
            None => skipped += 1,
        }
    }
    let primary_id = metadata
        .get("primaryId")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let revision = metadata.get("revision").and_then(Value::as_u64);
    Ok(ParsedEvents { events, skipped, primary_id, revision })
}

fn parse_event(item: &Value) -> Option<EventData> {
    let object = item.as_object()?;
    let name = object.get("name").and_then(|value| value.as_str()).unwrap_or("").to_string();
    let time = object.get("date").and_then(|value| value.as_str()).unwrap_or("").to_string();
    if name.is_empty() && time.is_empty() {
        return None;
    }
    Some(EventData {
        id: object.get("id").and_then(|value| value.as_str()).unwrap_or("").to_string(),
        name,
        time,
        // 腕端旧数据可能是字符串布尔，宽松解析避免整条被丢
        on_index: loose_bool(object.get("on_index")),
        if_staring_day: loose_bool(object.get("IFStaringDay")),
        theme_color: object.get("themeColor").and_then(|value| value.as_str()).unwrap_or("").to_string(),
    })
}

fn loose_bool(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Bool(flag)) => *flag,
        Some(Value::String(text)) => text == "true",
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use super::*;

    #[test]
    fn 名称含引号反斜杠换行时消息仍是合法JSON() {
        let draft = EventData::draft("考试,\"期末\"\n冲刺\\", "2026-03-05", true, false);
        let payload = add_event(&draft);
        let parsed: Value = serde_json::from_str(&payload).expect("必须是合法 JSON");
        assert_eq!(parsed["type"], "addEvent");
        assert_eq!(parsed["name"], "考试,\"期末\"\n冲刺\\");
        assert_eq!(parsed["date"], "2026-03-05");
        assert_eq!(parsed["on_index"], true);
        assert_eq!(parsed["IFStaringDay"], false);
    }

    #[test]
    fn 中文与emoji名称不转义丢失() {
        let draft = EventData::draft("生日 🎂", "2026-07-01", false, true);
        let parsed: Value = serde_json::from_str(&add_event(&draft)).unwrap();
        assert_eq!(parsed["name"], "生日 🎂");
    }

    #[test]
    fn 修改消息按稳定ID或下标二选一() {
        let draft = EventData::draft("改名", "2026-02-02", false, false);
        let by_id: Value = serde_json::from_str(&change_event(&draft, &EventRef::Id("e1".into()))).unwrap();
        assert_eq!(by_id["id"], "e1");
        assert!(by_id.get("index").is_none(), "带 ID 时不得再发下标");

        let by_index: Value = serde_json::from_str(&change_event(&draft, &EventRef::Index(3))).unwrap();
        assert_eq!(by_index["index"], 3);
        assert!(by_index.get("id").is_none(), "无 ID 时才发下标");

        let deleted: Value = serde_json::from_str(&delete_event(&EventRef::Id("e1".into()))).unwrap();
        assert_eq!(deleted["type"], "deleteEvent");
        assert_eq!(deleted["id"], "e1");
        assert_eq!(get_all_events(), r#"{"type":"getAllEvent"}"#);
    }

    #[test]
    fn 背景消息字段名与腕端一致() {
        let parsed: Value = serde_json::from_str(&add_background("YWJj")).unwrap();
        assert_eq!(parsed["type"], "addBG");
        assert_eq!(parsed["bgBase64"], "YWJj");
    }

    #[test]
    fn 解析腕端列表兼容包裹与数组两种形态() {
        let inner = json!({"data": [{"id": "e1", "name": "甲", "date": "2026-01-01", "on_index": true}]});
        let wrapped = json!({"payloadText": inner.to_string()}).to_string();
        let parsed = parse_event_list(&wrapped).expect("包裹形态可解析");
        assert_eq!(parsed.events.len(), 1);
        assert_eq!(parsed.events[0].id, "e1");
        assert!(parsed.events[0].on_index);

        let direct = json!({"data": [{"id": "e2", "name": "乙", "date": "2026-02-02"}]}).to_string();
        let parsed = parse_event_list(&direct).expect("直接形态可解析");
        assert_eq!(parsed.events[0].name, "乙");
        assert!(!parsed.events[0].on_index);
    }

    #[test]
    fn 解析快应用实际connect_envelope及表盘元数据() {
        let envelope = json!({
            "data": {
                "data": [{"id":"e1", "name":"甲", "date":"2026-01-01", "themeColor":"#3184d0"}],
                "primaryId": "e1",
                "revision": 42
            }
        }).to_string();
        let parsed = parse_event_list(&envelope).unwrap();
        assert_eq!(parsed.events.len(), 1);
        assert_eq!(parsed.events[0].id, "e1");
        assert_eq!(parsed.primary_id.as_deref(), Some("e1"));
        assert_eq!(parsed.revision, Some(42));

        let wrapped = json!({"payloadText": envelope}).to_string();
        let parsed = parse_event_list(&wrapped).unwrap();
        assert_eq!(parsed.primary_id.as_deref(), Some("e1"));

        let payload_wrapper = json!({"payload": envelope}).to_string();
        let parsed = parse_event_list(&payload_wrapper).unwrap();
        assert_eq!(parsed.revision, Some(42));
    }

    #[test]
    fn 解析异常返回可读原因() {
        assert_eq!(parse_event_list("not json"), Err(ParseIssue::NotJson));
        assert_eq!(parse_event_list(r#"{"x":1}"#), Err(ParseIssue::MissingData));
        assert_eq!(parse_event_list(r#"{"data":3}"#), Err(ParseIssue::NotArray));
        assert!(ParseIssue::NotJson.message().contains("JSON"));
    }

    #[test]
    fn 无效条目被跳过并计数不整体失败() {
        let payload = json!({
            "data": [
                {"id": "e1", "name": "甲", "date": "2026-01-01"},
                "字符串条目",
                {"id": "e2"},
                null,
                {"id": "e3", "name": "丙", "date": "2026-03-03", "on_index": "true", "IFStaringDay": "false"}
            ]
        })
        .to_string();
        let parsed = parse_event_list(&payload).unwrap();
        assert_eq!(parsed.events.len(), 2);
        assert_eq!(parsed.skipped, 3);
        assert!(parsed.events[1].on_index, "字符串布尔应宽松解析为 true");
    }

    #[test]
    fn 解析主事件与修订号() {
        let payload = json!({
            "data": [{"id": "e1", "name": "甲", "date": "2026-01-01"}],
            "primaryId": "e1",
            "revision": 7
        })
        .to_string();
        let parsed = parse_event_list(&payload).unwrap();
        assert_eq!(parsed.primary_id.as_deref(), Some("e1"));
        assert_eq!(parsed.revision, Some(7));
    }

    #[test]
    fn 保留主题色字段() {
        let payload = json!({
            "data": [{"id": "e1", "name": "甲", "date": "2026-01-01", "themeColor": "#f78803"}]
        })
        .to_string();
        let parsed = parse_event_list(&payload).unwrap();
        assert_eq!(parsed.events[0].theme_color, "#f78803");
    }
}
