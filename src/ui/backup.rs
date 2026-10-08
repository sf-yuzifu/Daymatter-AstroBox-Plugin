//! 完整备份、严格校验和恢复预览；不调用宿主。
use serde_json::{json, Value};
use super::{protocol, state::EventData};

pub const MAX_BYTES: usize = 240 * 1024;

fn canonical(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort_by_cached_key(|key| key.encode_utf16().collect::<Vec<_>>());
            format!("{{{}}}", keys.iter().map(|key| format!("{}:{}", json!(key), canonical(&map[*key]))).collect::<Vec<_>>().join(","))
        }
        Value::Array(items) => format!("[{}]", items.iter().map(canonical).collect::<Vec<_>>().join(",")),
        Value::Number(number) => { let n = number.as_f64().unwrap(); format!("n{:016x}", if n == 0.0 {0} else {n.to_bits()}) },
        _ => value.to_string(),
    }
}

pub fn checksum(data: &Value) -> String {
    let hash = canonical(data).encode_utf16().fold(5381u32, |h, c| h.wrapping_mul(33).wrapping_add(c as u32));
    format!("{hash:08x}")
}

pub fn event_value(event: &EventData) -> Value {
    let mut value = json!(event.extensions);
    for (key, field) in [("id", json!(event.id)), ("name", json!(event.name)), ("date", json!(event.time)),
        ("on_index", json!(event.on_index)), ("IFStaringDay", json!(event.if_staring_day)), ("themeColor", json!(event.theme_color))] {
        value[key] = field;
    }
    value
}

pub fn make(events: &[EventData], primary: &str, sort: &str) -> Result<Value, &'static str> {
    let data = json!({"version":2, "primaryId":primary, "sortMode":sort,
        "events":events.iter().map(event_value).collect::<Vec<_>>()});
    let backup = json!({"format":"daymatter-backup", "backupVersion":1, "checksum":checksum(&data), "data":data});
    validate(&backup)?;
    Ok(backup)
}

pub fn validate(backup: &Value) -> Result<(), &'static str> {
    if backup.to_string().len() > MAX_BYTES { return Err("备份超过 240 KiB 预算"); }
    if backup["format"] != "daymatter-backup" || backup["backupVersion"] != 1 || backup["data"]["version"] != 2 {
        return Err("不支持的备份格式或数据版本");
    }
    let data = &backup["data"];
    if backup["checksum"].as_str() != Some(checksum(data).as_str()) { return Err("备份完整性校验失败"); }
    let events = data["events"].as_array().ok_or("备份缺少事件数组")?;
    let parsed = protocol::parse_event_list(&json!({"data":events,"primaryId":data["primaryId"],"sortMode":data["sortMode"]}).to_string())
        .map_err(|_| "备份事件、身份或排序字段无效")?;
    if parsed.skipped != 0 || parsed.events.iter().any(|e| e.id.is_empty()) { return Err("备份包含无效事件或缺少稳定 ID"); }
    for e in events {
        if !["on_index", "IFStaringDay"].iter().all(|key| e[key].is_boolean()) { return Err("备份显示字段必须为布尔值"); }
        if !e["id"].as_str().is_some_and(|id| id.len() <= 128) { return Err("备份 ID 超过预算"); }
        if e["themeColor"].as_str().is_none() { return Err("备份缺少颜色字段"); }
    }
    let primary = data["primaryId"].as_str().ok_or("备份缺少主事件引用")?;
    if !primary.is_empty() && !parsed.events.iter().any(|e| e.id == primary && e.extensions.get("archived").and_then(Value::as_bool) != Some(true)) {
        return Err("备份主事件不存在或已归档");
    }
    if data["sortMode"].as_str().is_none() { return Err("备份缺少排序模式"); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(id: &str) -> EventData {
        let mut event = EventData::draft("同名😀", "2020-02-29", true, false);
        event.id = id.into();
        event.extensions.insert("repeat".into(), json!("yearly"));
        event.extensions.insert("displayUnit".into(), json!("weeks"));
        event.extensions.insert("pinned".into(), json!(true));
        event.extensions.insert("sortOrder".into(), json!(4));
        event.extensions.insert("custom".into(), json!({"nested":[null, 1.25, -0.0, "引号\"\\\n"]}));
        event
    }

    #[test]
    fn complete_backup_preview_conflicts_and_integrity() {
        let backup = make(&[event("a"), event("b")], "a", "manual").unwrap();
        validate(&backup).unwrap();
        assert_eq!(backup["data"]["events"][0]["custom"]["nested"][1], 1.25);
        let preview = Preview::new(backup.clone(), &[event("a"), event("c")], "watch".into(), 10).unwrap();
        assert_eq!(preview.conflicts.len(), 1);
        assert_eq!(preview.same_names.len(), 2);
        assert_eq!(preview.merged_count, 3);
        let payload: Value = serde_json::from_str(&preview.payload().unwrap()).unwrap();
        assert_eq!(payload["mode"], "merge");
        assert_eq!(payload["conflict"], "keep");
        let mut corrupt = backup.clone();
        corrupt["data"]["events"][0]["name"] = json!("改动");
        assert!(validate(&corrupt).is_err());
        for mut data in [backup["data"].clone()] {
            data["primaryId"] = json!("missing");
            let invalid = json!({"format":"daymatter-backup","backupVersion":1,"checksum":checksum(&data),"data":data});
            assert!(validate(&invalid).is_err());
        }
        let mut duplicate = backup["data"].clone();
        duplicate["events"][1]["id"] = json!("a");
        assert!(validate(&json!({"format":"daymatter-backup","backupVersion":1,"checksum":checksum(&duplicate),"data":duplicate})).is_err());
    }

    #[test]
    fn preview_requires_fresh_complete_snapshot_and_keeps_unknown_recovery_entry() {
        use crate::ui::{state::{UiState, EventType, SendPurpose}, view, dispatch::{UiId, Action, apply_action}};
        let mut state = UiState::new("2026-10-08".into());
        state.current_tab = EventType::ModifyEvent;
        state.all_events = vec![event("a")];
        assert!(state.export_backup().is_err());
        state.snapshot_complete = true;
        state.snapshot_token = Some(1);
        state.target_device = Some("watch".into());
        state.snapshot_device = state.target_device.clone();
        state.primary_id = Some("a".into());
        state.revision = Some(10);
        let backup = state.export_backup().unwrap();
        state.preview_backup(backup).unwrap();
        state.modify_event_data.name = "未保存".into();
        let view = view::build(&state);
        assert!(view.bound_ids().contains(&UiId::BackupMode.as_str()));
        assert!(view.bound_ids().contains(&UiId::RestoreBackup.as_str()));
        state.revision = Some(11);
        assert!(state.restore_payload().is_err());
        state.revision = Some(10);
        let token = state.reserve_send(state.restore_payload().unwrap(), SendPurpose::Mutation).unwrap();
        state.attach_send_device(token, "watch".into());
        let pending = state.claim_send(token).unwrap();
        state.begin_result_wait(&pending);
        apply_action(&mut state, Action::BackupMode);
        assert!(!state.backup_preview.as_ref().unwrap().replace);
        assert!(state.apply_request_message(&json!({"type":"mutationResult","requestId":pending.request_id,
            "sessionId":pending.session_id,"deviceId":"watch","ok":false,"code":"SAVE_FAIL"})));
        assert!(state.backup_preview.is_some());
        assert_eq!(state.modify_event_data.name, "未保存");
        state.snapshot_token = None;
        assert!(state.restore_payload().is_err());
    }
}

#[derive(Clone)]
pub struct Preview {
    pub backup: Value,
    pub device: String,
    pub revision: u64,
    pub replace: bool,
    pub overwrite: bool,
    pub conflicts: Vec<String>,
    pub same_names: Vec<String>,
    pub merged_count: usize,
}

impl Preview {
    pub fn new(backup: Value, events: &[EventData], device: String, revision: u64) -> Result<Self, &'static str> {
        validate(&backup)?;
        let incoming = backup["data"]["events"].as_array().unwrap();
        let conflicts = incoming.iter().filter(|e| events.iter().any(|old| old.id == e["id"].as_str().unwrap()))
            .map(|e| format!("{} · {}", e["name"].as_str().unwrap(), e["id"].as_str().unwrap())).collect();
        let same_names = incoming.iter().filter(|e| events.iter().any(|old| old.name == e["name"].as_str().unwrap() && old.id != e["id"].as_str().unwrap()))
            .map(|e| format!("{} · {}", e["name"].as_str().unwrap(), e["id"].as_str().unwrap())).collect();
        let merged_count = events.len() + incoming.iter().filter(|e| !events.iter().any(|old| old.id == e["id"].as_str().unwrap())).count();
        Ok(Self {backup, device, revision, replace:false, overwrite:false, conflicts, same_names, merged_count})
    }

    pub fn payload(&self) -> Result<String, &'static str> {
        if !self.replace && self.merged_count > protocol::MAX_LIST_EVENTS { return Err("合并后超过 200 条，请调整备份或选择替换"); }
        Ok(json!({"type":"restoreBackup", "backup":self.backup, "expectedRevision":self.revision,
            "mode":if self.replace {"replace"} else {"merge"}, "conflict":if self.overwrite {"overwrite"} else {"keep"}}).to_string())
    }
}
