//! 有界停等发送：ACK只推进当前帧，纯逻辑可在native与真实腕端桥接中验证。
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};
use super::state::{SendPurpose, UiState};

pub const CHUNK_BYTES: usize = 3072;

pub fn checksum(bytes: &[u8]) -> u32 {
    bytes.iter().fold(2166136261u32, |hash, byte| (hash ^ u32::from(*byte)).wrapping_mul(16777619))
}

pub fn accept_ack(state: &mut UiState, value: &Value) -> Option<(String, String, usize, usize)> {
    let request = state.result_request.as_ref()?.clone();
    if request.purpose != SendPurpose::Background || state.background_bytes.is_empty()
        || value["sessionId"].as_str() != Some(request.session_id.as_str())
        || value["requestId"].as_str() != Some(request.request_id.as_str())
        || value["deviceId"].as_str() != Some(request.device_id.as_str()) { return None; }
    let frame: Value = serde_json::from_str(state.background_frame.as_ref()?).ok()?;
    let next = usize::try_from(value["next"].as_u64()?).ok()?;
    let expected = match frame["type"].as_str()? {
        "beginBG" => 0,
        "backgroundChunk" => (state.background_offset + CHUNK_BYTES).min(state.background_bytes.len()),
        // finish正在等待最终回执，迟到最后一片ACK不能再次推进。
        _ => return None,
    };
    if next != expected { return None; }
    state.background_offset = next;
    state.background_retries = 0;
    let mut outgoing = if next == state.background_bytes.len() { json!({"type":"finishBG"}) }
    else {
        let end = (next + CHUNK_BYTES).min(state.background_bytes.len());
        json!({"type":"backgroundChunk", "offset":next, "data":STANDARD.encode(&state.background_bytes[next..end])})
    };
    outgoing["sessionId"] = json!(request.session_id);
    outgoing["requestId"] = json!(request.request_id);
    outgoing["deviceId"] = json!(request.device_id);
    let payload = outgoing.to_string();
    state.background_frame = Some(payload.clone());
    Some((request.device_addr, payload, next, state.background_bytes.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn early_duplicate_wrong_identity_and_finish_ack_are_bounded() {
        let mut state = UiState::new("2026-10-09".into());
        state.session_id = "s".into();
        state.background_device = Some("d".into());
        let token = state.reserve_send(json!({"type":"beginBG"}).to_string(), SendPurpose::Background).unwrap();
        assert!(state.attach_send_device(token, "d".into()));
        let request = state.pending_send.clone().unwrap();
        state.begin_result_wait(&request);
        state.background_bytes = vec![1; 4000];
        state.background_frame = Some(request.payload.clone());
        let ack = |next| json!({"sessionId":request.session_id,"requestId":request.request_id,"deviceId":"d","next":next});
        let mut wrong = ack(0); wrong["deviceId"] = json!("other");
        assert!(accept_ack(&mut state, &wrong).is_none());
        assert!(accept_ack(&mut state, &ack(0)).is_some()); // transport callback尚未到达
        assert!(accept_ack(&mut state, &ack(0)).is_none());
        assert!(accept_ack(&mut state, &ack(4000)).is_none());
        assert!(accept_ack(&mut state, &ack(3072)).is_some());
        assert!(accept_ack(&mut state, &ack(3072)).is_none());
        assert!(accept_ack(&mut state, &ack(4000)).is_some());
        assert!(accept_ack(&mut state, &ack(4000)).is_none());
        assert_eq!(serde_json::from_str::<Value>(state.background_frame.as_ref().unwrap()).unwrap()["type"], "finishBG");
    }
}
