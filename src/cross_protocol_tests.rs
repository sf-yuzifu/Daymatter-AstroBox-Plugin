use crate::ui::{protocol, state::{EventData, SendPurpose, UiState}};
use serde_json::{json, Value};
use std::{io::Write, process::{Command, Stdio}};

fn wrist(input: Value) -> Value {
    let bridge = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../daymatter/tools/protocol-bridge.cjs");
    assert!(bridge.exists(), "E-14 requires sibling daymatter repository");
    let mut child = Command::new("node").arg(bridge).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    child.stdin.take().unwrap().write_all(input.to_string().as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn sorting_archive_fields_move_receipt_and_old_patch_roundtrip() {
    let mut a = EventData::draft("生日", "2020-02-29", true, false);
    a.extensions.insert("pinned".into(), json!(true));
    a.extensions.insert("category".into(), json!("birthday"));
    a.extensions.insert("archived".into(), json!(true));
    let output = wrist(json!({"events":[
        {"id":"a", "name":"甲", "date":"2026-12-01", "on_index":true},
        {"id":"b", "name":"乙", "date":"2026-12-01", "on_index":true}
    ], "messages":[protocol::capabilities("cross-session"), protocol::add_event(&a),
        protocol::set_request_identity_for(&json!({"type":"moveEvent", "id":"b", "direction":"up"}).to_string(), "move", "cross-session", "watch"),
        json!({"type":"changeEvent", "index":2, "name":"旧端改名"}).to_string(), protocol::get_all_events()]}));
    assert_eq!(output["store"]["sortMode"], "manual");
    assert_eq!(output["store"]["events"][1]["sortOrder"], 0);
    let result = output["messages"].as_array().unwrap().iter().find(|m| m["requestId"] == "move" && m["type"] == "mutationResult").unwrap();
    assert_eq!(result["ok"], true);
    assert_eq!(result["sortMode"], "manual");
    let list = output["messages"].as_array().unwrap().iter().find(|m| m.get("data").is_some()).unwrap();
    let parsed = protocol::parse_event_list(&list.to_string()).unwrap();
    assert_eq!(parsed.sort_mode.as_deref(), Some("manual"));
    let event = &parsed.events[2];
    assert_eq!(event.extensions["pinned"], true);
    assert_eq!(event.extensions["archived"], true);
    assert_eq!(event.extensions["category"], "birthday");
    assert_eq!(event.name, "旧端改名");
}

fn request(purpose: SendPurpose) -> (UiState, String) {
    let mut state = UiState::new("2026-10-07".into());
    state.session_id = "cross-session".into();
    let payload = if purpose == SendPurpose::Fetch { protocol::get_all_events() }
        else { protocol::add_event(&EventData::draft("甲\"\\\n🎂", "2026-12-01", true, false)) };
    let token = state.reserve_send(payload.clone(), purpose).unwrap();
    state.attach_send_device(token, "watch".into());
    let pending = state.claim_send(token).unwrap();
    let mut tagged: Value = serde_json::from_str(&protocol::set_request_identity_for(&payload,
        &pending.request_id, &pending.session_id, &pending.device_id)).unwrap();
    if purpose == SendPurpose::Fetch {
        state.begin_list_wait(&pending, true);
        tagged["listBatches"] = json!(true);
    } else { state.begin_result_wait(&pending); }
    (state, tagged.to_string())
}

#[test]
fn actual_rust_request_wrist_inflight_retry_query_and_early_receipt() {
    let (mut state, request) = request(SendPurpose::Mutation);
    let query = protocol::request_status_for("req-1", "cross-session", "watch");
    let output = wrist(json!({"deferWrites":true, "messages":[protocol::capabilities("cross-session"), request, request, query]}));
    assert_eq!(output["store"]["events"].as_array().unwrap().len(), 1);
    assert_eq!(output["store"]["events"][0]["name"], "甲\"\\\n🎂");
    let messages = output["messages"].as_array().unwrap();
    assert!(messages.iter().any(|m| m["status"] == "processing"));
    let result = messages.iter().find(|m| m["type"] == "mutationResult").unwrap();
    let wrapped = json!({"payloadText":json!({"data":result}).to_string()});
    let decoded = protocol::decode_message(&wrapped.to_string()).unwrap();
    assert!(state.apply_request_message(&decoded));
    assert!(!state.busy());
    assert!(state.finish_send(1).is_none());
    assert!(!state.apply_request_message(&decoded));
}

fn events() -> Vec<Value> {
    (0..200).map(|i| json!({"id":format!("e{i}"),"name":format!("甲\"\\\n🎂 {i}"),
        "date":"2026-12-01","on_index":true,"IFStaringDay":false})).collect()
}

#[test]
fn recurring_fields_roundtrip_and_old_patch_preserves_preferences() {
    let mut draft = EventData::draft("生日", "2020-02-29", true, false);
    draft.extensions.insert("repeat".into(), json!("yearly"));
    draft.extensions.insert("displayUnit".into(), json!("weeks"));
    let output = wrist(json!({"messages":[protocol::add_event(&draft),
        {"type":"changeEvent","index":0,"name":"生日改名"}, protocol::get_all_events()]}));
    let list = output["messages"].as_array().unwrap().iter().find(|m| m.get("data").is_some()).unwrap();
    let parsed = protocol::parse_event_list(&list.to_string()).unwrap();
    let event = &parsed.events[0];
    assert_eq!(event.time, "2020-02-29");
    assert_eq!(event.extensions["repeat"], "yearly");
    assert_eq!(event.extensions["displayUnit"], "weeks");
    let patch: Value = serde_json::from_str(&protocol::change_event(event, &protocol::EventRef::Id(event.id.clone()))).unwrap();
    assert_eq!(patch["repeat"], "yearly");
    assert_eq!(patch["displayUnit"], "weeks");
    assert_eq!(crate::ui::validation::occurrence(&event.time, "yearly", "2026-02-28"), Some(("2026-02-28".into(), Some(6))));
    assert_eq!(crate::ui::validation::occurrence(&event.time, "yearly", "2028-02-28"), Some(("2028-02-29".into(), Some(8))));
}

#[test]
fn name_rules_match_actual_wrist_for_unicode_and_whitespace() {
    use crate::ui::validation::{validate_name, normalized_name};
    for name in ["😀".repeat(50), "😀".repeat(51), "e\u{0301}".repeat(25),
        "e\u{0301}".repeat(26), "\u{feff}\u{3000}\t".into(), " a  b\u{0085} ".into()] {
        let expected = validate_name(&name, None).is_ok();
        let draft = EventData::draft(&name, "2026-12-01", true, false);
        let payload = protocol::set_request_identity_for(&protocol::add_event(&draft), "name-test", "cross-session", "watch");
        let output = wrist(json!({"messages":[protocol::capabilities("cross-session"), payload]}));
        let result = output["messages"].as_array().unwrap().iter().find(|m| m["type"] == "mutationResult").unwrap();
        assert_eq!(result["ok"], expected, "{}", name);
        if expected { assert_eq!(output["store"]["events"][0]["name"], normalized_name(&name, None)); }
    }
}

#[test]
fn actual_wrist_batches_rust_reverse_duplicate_stale_device_and_drafts() {
    let (mut state, request) = request(SendPurpose::Fetch);
    state.event_data.name = "草稿".into();
    let expected = events();
    let output = wrist(json!({"events":expected,"messages":[protocol::capabilities("cross-session"),request]}));
    let batches: Vec<_> = output["messages"].as_array().unwrap().iter().filter(|m| m["type"] == "eventListBatch").collect();
    assert!(batches.len() > 1);
    let mut stale = batches[0].clone();
    stale["deviceId"] = json!("other");
    assert!(!state.accept_list_batch(&stale));
    stale["deviceId"] = json!("watch");
    stale["requestId"] = json!("old");
    assert!(!state.accept_list_batch(&stale));
    for (index, batch) in batches.iter().rev().enumerate() {
        let decoded = protocol::decode_message(&json!({"payload":json!({"data":batch}).to_string()}).to_string()).unwrap();
        assert_eq!(state.accept_list_batch(&decoded), index == batches.len() - 1);
        assert!(!state.accept_list_batch(&decoded));
    }
    assert_eq!(state.all_events.len(), 200);
    for (event, expected) in state.all_events.iter().zip(expected.iter()) { assert_eq!(event.name, expected["name"].as_str().unwrap()); }
    assert_eq!(state.event_data.name, "草稿");
}

#[test]
fn actual_wrist_send_failure_preserves_rust_snapshot() {
    let (mut state, request) = request(SendPurpose::Fetch);
    state.all_events.push(EventData::draft("旧数据", "2026-01-01", false, false));
    let output = wrist(json!({"events":events(),"failBatch":1,
        "messages":[protocol::capabilities("cross-session"),request]}));
    let mut failed = false;
    for message in output["messages"].as_array().unwrap() {
        if message["type"] == "eventListBatch" { state.accept_list_batch(message); }
        if message["type"] == "eventListError" { failed = state.accept_list_batch(message); }
    }
    assert!(failed);
    assert_eq!(state.all_events[0].name, "旧数据");
    assert!(state.list_request.is_none());
    assert!(state.list_assembly.is_none());
}

#[test]
fn legacy_wrist_list_decodes_and_mutation_query_replays_original_result() {
    let (mut state, request) = request(SendPurpose::Mutation);
    let output = wrist(json!({"messages":[protocol::capabilities("cross-session"),request,
        protocol::request_status_for("req-1","cross-session","watch"),protocol::get_all_events()]}));
    let results: Vec<_> = output["messages"].as_array().unwrap().iter().filter(|m| m["type"] == "mutationResult").collect();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0], results[1]);
    state.result_deadline = None;
    state.finish_send(1);
    assert!(state.apply_request_message(results[1]), "查询后迟到结果仍结算");
    let list = output["messages"].as_array().unwrap().iter().find(|m| m.get("data").is_some()).unwrap();
    let parsed = protocol::parse_event_list(&json!({"data":list}).to_string()).unwrap();
    assert_eq!(parsed.events.len(), 1);
    assert_eq!(parsed.events[0].name, "甲\"\\\n🎂");
}

#[test]
fn accumulated_batch_budget_rejects_without_replacing_snapshot() {
    let (mut state, _) = request(SendPurpose::Fetch);
    state.all_events.push(EventData::draft("旧数据", "2026-01-01", false, false));
    for i in 0..200 {
        let batch = json!({"type":"eventListBatch","requestId":"req-1","sessionId":"cross-session",
            "deviceId":"watch","revision":10,"primaryId":"","total":200,"batchCount":200,
            "batchIndex":i,"data":[{"id":format!("e{i}"),"name":"x".repeat(2000),"date":"2026-01-01"}]});
        assert!(batch.to_string().len() < 8192);
        if state.accept_list_batch(&batch) { break; }
    }
    assert!(state.list_request.is_none());
    assert_eq!(state.all_events[0].name, "旧数据");
}

#[test]
fn actual_handshake_echo_is_bound_to_operation_and_device() {
    let (mut state, _) = request(SendPurpose::Fetch);
    state.start_handshake(1);
    let output = wrist(json!({"messages":[protocol::handshake("cross-session",1,"watch")]}));
    let reply = &output["messages"][0];
    assert_eq!(reply["handshakeId"], "hello-1");
    assert_eq!(reply["deviceId"], "watch");
    assert!(state.accept_capabilities(reply));
    assert!(state.supports("listBatches"));
}

#[test]
fn modify_preserves_wrist_color_extensions_and_list_metadata() {
    let item = json!({"id":"e1","name":"旧事件","date":"2026-01-01","on_index":false,
        "IFStaringDay":true,"themeColor":"#e74c3c","futureRule":{"kind":"annual"}});
    let payload = protocol::change_event(&EventData::draft("改名", "2026-02-01", true, false), &protocol::EventRef::Id("e1".into()));
    let output = wrist(json!({"events":[item], "messages":[payload,protocol::get_all_events()]}));
    let event = &output["store"]["events"][0];
    assert_eq!(event["themeColor"], "#e74c3c");
    assert_eq!(event["futureRule"]["kind"], "annual");
    let list = output["messages"].as_array().unwrap().iter().find(|m| m.get("data").is_some()).unwrap();
    let parsed = protocol::parse_event_list(&list.to_string()).unwrap();
    assert_eq!(parsed.events[0].extensions["futureRule"]["kind"], "annual");
    assert_eq!(parsed.events[0].theme_color, "#e74c3c");
}
