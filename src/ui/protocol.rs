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
    TooLarge,
    TooManyEvents,
    InvalidMetadata,
    DuplicateId,
}

impl ParseIssue {
    pub fn message(self) -> &'static str {
        match self {
            ParseIssue::NotJson => "手环端返回不是合法 JSON",
            ParseIssue::MissingData => "手环端返回缺少 data 字段",
            ParseIssue::NotArray => "手环端返回的 data 不是数组",
            ParseIssue::TooLarge => "腕端消息超过接收预算，请更新或减少数据",
            ParseIssue::TooManyEvents => "腕端事件超过 200 条，未替换现有列表",
            ParseIssue::InvalidMetadata => "腕端列表身份或修订号字段无效",
            ParseIssue::DuplicateId => "腕端列表包含重复 ID，不能安全操作",
        }
    }
}

pub const PKG_NAME: &str = "com.yzf.daymatter";
pub const MIN_APP_VERSION_CODE: u32 = 21000;
pub const PROTOCOL_VERSION: u32 = 2;
pub const MAX_MESSAGE_BYTES: usize = 256 * 1024;
pub const MAX_LIST_EVENTS: usize = 200;

pub fn new_session_id(sequence: u64) -> String {
    format!("abp-{}-{}", std::process::id(), sequence)
}

pub fn capabilities(session_id: &str) -> String {
    json!({"type":"hello", "protocolVersion":PROTOCOL_VERSION, "sessionId":session_id,
        "capabilities":["stableId","mutationResult","requestId","listRequestId","themeColor","primaryId","revision","backgroundJpeg"]}).to_string()
}

pub fn request_status(request_id: &str, session_id: &str) -> String {
    json!({"type":"getRequestResult", "requestId":request_id, "sessionId":session_id}).to_string()
}

pub fn request_status_for(request_id: &str, session_id: &str, device_id: &str) -> String {
    json!({"type":"getRequestResult", "requestId":request_id, "sessionId":session_id, "deviceId":device_id}).to_string()
}

pub fn set_request_identity(payload: &str, request_id: &str, session_id: &str) -> String {
    set_request_identity_for(payload, request_id, session_id, "")
}

pub fn set_request_identity_for(payload: &str, request_id: &str, session_id: &str, device_id: &str) -> String {
    let mut value: Value = serde_json::from_str(payload).unwrap_or_else(|_| json!({}));
    if let Some(object) = value.as_object_mut() {
        object.insert("protocolVersion".into(), json!(PROTOCOL_VERSION));
        object.insert("requestId".into(), json!(request_id));
        object.insert("sessionId".into(), json!(session_id));
        if !device_id.is_empty() { object.insert("deviceId".into(), json!(device_id)); }
    }
    value.to_string()
}

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

/// 控制消息与列表共用宿主包装解码，最多展开四层。
pub fn decode_message(payload: &str) -> Result<Value, ParseIssue> {
    if payload.len() > MAX_MESSAGE_BYTES { return Err(ParseIssue::TooLarge); }
    let mut value: Value = serde_json::from_str(payload).map_err(|_| ParseIssue::NotJson)?;
    for _ in 0..4 {
        if let Some(text) = value.get("payloadText").or_else(|| value.get("payload")).and_then(Value::as_str) {
            value = serde_json::from_str(text).map_err(|_| ParseIssue::NotJson)?;
        } else if value.get("type").is_none() && value.get("data").is_some_and(Value::is_object) {
            value = value["data"].clone();
        } else { break; }
    }
    Ok(value)
}

/// 解析腕端事件列表：兼容 `payloadText` 包裹；单条异常只跳过该条并计数
#[derive(Debug, PartialEq)]
pub struct ParsedEvents {
    pub events: Vec<EventData>,
    pub skipped: usize,
    pub primary_id: Option<String>,
    pub revision: Option<u64>,
    pub request_id: Option<String>,
    pub session_id: Option<String>,
    pub device_id: Option<String>,
}

pub fn parse_event_list(payload: &str) -> Result<ParsedEvents, ParseIssue> {
    let inner = decode_message(payload)?;
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
            let metadata = if map.get("primaryId").is_some() || map.get("revision").is_some() || map.get("requestId").is_some() {
                data
            } else {
                &inner
            };
            (array, metadata)
        }
        _ => return Err(ParseIssue::NotArray),
    };

    if array.len() > MAX_LIST_EVENTS { return Err(ParseIssue::TooManyEvents); }
    for field in ["requestId", "sessionId", "deviceId", "primaryId"] {
        if let Some(value) = metadata.get(field) {
            if !value.as_str().is_some_and(|text| text.len() <= 256 && (field == "primaryId" || !text.is_empty())) {
                return Err(ParseIssue::InvalidMetadata);
            }
        }
    }
    if metadata.get("revision").is_some_and(|value| value.as_u64().is_none_or(|n| n > 9_007_199_254_740_991)) {
        return Err(ParseIssue::InvalidMetadata);
    }
    // 即使某条事件其他字段无效，也不让相同 ID 的另一条被当作唯一身份。
    let mut ids = std::collections::HashSet::new();
    for item in array {
        if let Some(id) = item.get("id").and_then(Value::as_str).filter(|id| !id.is_empty()) {
            if !ids.insert(id) { return Err(ParseIssue::DuplicateId); }
        }
    }
    let mut events = Vec::new();
    let mut skipped = 0;
    for (index, item) in array.iter().enumerate() {
        match parse_event(item, index) {
            Some(event) => events.push(event),
            None => skipped += 1,
        }
    }
    let primary_id = metadata
        .get("primaryId")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let revision = metadata.get("revision").and_then(Value::as_u64);
    let request_id = metadata.get("requestId").and_then(Value::as_str).map(str::to_owned);
    let session_id = metadata.get("sessionId").and_then(Value::as_str).map(str::to_owned);
    let device_id = metadata.get("deviceId").and_then(Value::as_str).map(str::to_owned);
    Ok(ParsedEvents { events, skipped, primary_id, revision, request_id, session_id, device_id })
}

fn parse_event(item: &Value, index: usize) -> Option<EventData> {
    let object = item.as_object()?;
    let name = object.get("name")?.as_str()?.to_string();
    let time = object.get("date")?.as_str()?.to_string();
    // 保留旧长名称，不静默截断；单字段仍有接收预算。
    if name.trim().is_empty() || name.len() > 4096 || crate::ui::validation::validate_date_format(&time).is_err() { return None; }
    let id = optional_text(object.get("id"), 256)?;
    let theme_color = optional_text(object.get("themeColor"), 16)?;
    if !theme_color.is_empty() && !["#3184d0", "#f78803", "#e74c3c", "#27ae60", "#9b59b6"].contains(&theme_color.as_str()) { return None; }
    Some(EventData {
        id,
        name,
        time,
        // 腕端旧数据可能是字符串布尔，宽松解析避免整条被丢
        on_index: checked_bool(object.get("on_index"))?,
        if_staring_day: checked_bool(object.get("IFStaringDay"))?,
        theme_color,
        source_index: Some(index),
    })
}

fn optional_text(value: Option<&Value>, max: usize) -> Option<String> {
    match value {
        None => Some(String::new()),
        Some(value) => value.as_str().filter(|text| text.len() <= max).map(str::to_owned),
    }
}

fn checked_bool(value: Option<&Value>) -> Option<bool> {
    match value {
        Some(Value::Bool(flag)) => Some(*flag),
        Some(Value::String(text)) if text == "true" || text == "false" => Some(text == "true"),
        None => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use super::*;

    #[test]
    fn 跳过异常条目仍按原数组下标选择修改删除() {
        let parsed = parse_event_list(&json!({"data":[null,
            {"name":"合法", "date":"2026-01-01"},
            {"name":"非法日期", "date":"2026-02-30"},
            {"name":"另一条", "date":"2026-03-01"}]}).to_string()).unwrap();
        assert_eq!(parsed.skipped, 2);
        assert_eq!(parsed.events[0].selection_key(0), "idx:1");
        assert_eq!(parsed.events[1].selection_key(1), "idx:3");
        let mut state = crate::ui::state::UiState::new("2026-10-07".into());
        state.all_events = parsed.events;
        assert_eq!(state.index_of_key("idx:3"), Some(1));
        assert_eq!(state.index_of_key("idx:0"), None);
        let command: Value = serde_json::from_str(&delete_event(&EventRef::Index(3))).unwrap();
        assert_eq!(command["index"], 3);
    }

    #[test]
    fn 严格字段校验保留旧长名称且无效项跳过() {
        let long_name = "旧".repeat(60);
        let parsed = parse_event_list(&json!({"data":[
            {"name":long_name,"date":"2026-01-01","on_index":"true"},
            {"name":123,"date":"2026-01-01"},
            {"name":"a","date":"中文日期ab"},
            {"name":"a","date":"2026-01-01","on_index":"yes"},
            {"name":"a","date":"2026-01-01","themeColor":"#ffffff"},
            {"name":"a","date":"2026-01-01","id":42}
        ]}).to_string()).unwrap();
        assert_eq!(parsed.events.len(), 1);
        assert_eq!(parsed.skipped, 5);
        assert_eq!(parsed.events[0].name, long_name);
    }

    #[test]
    fn 重复ID和畸形元数据拒绝整批() {
        let item = json!({"id":"same","name":"甲","date":"2026-01-01"});
        assert_eq!(parse_event_list(&json!({"data":[item,item]}).to_string()), Err(ParseIssue::DuplicateId));
        for metadata in [json!({"revision":-1}), json!({"revision":"1"}),
            json!({"requestId":3}), json!({"sessionId":""})] {
            let mut value = metadata;
            value["data"] = json!([]);
            assert_eq!(parse_event_list(&value.to_string()), Err(ParseIssue::InvalidMetadata));
        }
    }

    #[test]
    fn 接收字节与数量预算在构造列表前拒绝() {
        assert_eq!(decode_message(&" ".repeat(MAX_MESSAGE_BYTES + 1)), Err(ParseIssue::TooLarge));
        assert_eq!(parse_event_list(&json!({"data":vec![Value::Null;201]}).to_string()), Err(ParseIssue::TooManyEvents));
        assert_eq!(parse_event_list(&json!({"data":vec![Value::Null;200]}).to_string()).unwrap().skipped, 200);
    }

    #[test]
    fn 控制回包统一解开宿主包装() {
        let result = json!({"type":"mutationResult", "requestId":"req-1", "ok":true});
        for wrapped in [result.clone(), json!({"data":result}),
            json!({"payloadText":json!({"data":result}).to_string()}),
            json!({"payload":result.to_string()})] {
            assert_eq!(decode_message(&wrapped.to_string()).unwrap(), result);
        }
    }

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
    fn 能力协商与请求身份序列化可往返() {
        let hello: Value = serde_json::from_str(&capabilities("session-x")).unwrap();
        assert_eq!(hello["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(hello["sessionId"], "session-x");
        assert!(hello["capabilities"].as_array().unwrap().iter().any(|v| v == "requestId"));
        let tagged: Value = serde_json::from_str(&set_request_identity(&add_event(&EventData::draft("🎂", "2026-07-01", true, false)), "req-8", "session-x")).unwrap();
        assert_eq!(tagged["requestId"], "req-8");
        assert_eq!(tagged["sessionId"], "session-x");
        let query: Value = serde_json::from_str(&request_status("req-8", "session-x")).unwrap();
        assert_eq!(query["type"], "getRequestResult");
        assert_eq!(query["requestId"], "req-8");
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
