//! 宿主交互：状态条、定时器、设备与快应用通道（E-04 / E-17 / E-19）。
//!
//! 等待一律由宿主定时器事件推进：`timer.set_timeout` 只注册并立即返回，
//! 插件里没有 async sleep，因此「启动快应用 → 再发消息」拆成启动 + 定时器两步，
//! 取代旧实现的 `thread::sleep(2 秒)`（会阻塞整个 guest，界面无响应）。

use crate::astrobox::psys_host_v4::{device, dialog, interconnect, os, register, thirdpartyapp, timer};
use crate::ui::build;
use crate::ui::dispatch;
use crate::ui::protocol::{self, MIN_APP_VERSION_CODE, PKG_NAME};
use crate::ui::state::{local_date, read_state, write_state, PendingSend, SendPurpose, StatusKind};
use chrono::Utc;

pub const HIDE_MESSAGE_TIMER: &str = "hide_message";
pub const SEND_TIMER: &str = "send_pending:";
/// 启动快应用后等待其完成加载，再发送互联消息
pub const APP_LAUNCH_SETTLE_MS: u64 = 1500;
pub const MAX_SEND_ATTEMPTS: u32 = 3;
pub const RETRY_INTERVAL_MS: u64 = 800;
pub const RESULT_TIMEOUT_MS: u64 = 8000;
pub const RESULT_TIMER: &str = "request_result:";
pub const HANDSHAKE_POLL_MS: u64 = 250;
pub const HANDSHAKE_MAX_POLLS: u32 = 8;
pub const LIST_TIMER: &str = "list_result:";
pub const SNAPSHOT_TIMER: &str = "snapshot_expire:";
pub const LEGACY_SNAPSHOT_MS: u64 = 60000;

/// 状态面板常驻：成功 / 失败不再自动消失，用户可对照腕端核实（P1-51）
///
/// 注意：只取一次状态锁，渲染通过 `build::render_with` 复用同一个 guard，
/// 不可在持锁时调用 `build::refresh`（RwLock 不可重入会死锁，界面将整体失去响应）。
pub fn set_status(kind: StatusKind, text: &str) {
    let request = {
        let mut state = write_state();
        state.message_timer_id = None;
        state.status = Some(crate::ui::state::StatusMessage { kind, text: text.to_string() });
        build::prepare_with(&mut state, false)
    };
    build::render_prepared(request);
}

/// 进行中提示：同样常驻，直到下一步状态覆盖
pub fn set_status_timed(kind: StatusKind, text: &str) {
    set_status(kind, text);
}

pub fn hide_message() {
    let request = {
        let mut state = write_state();
        state.message_timer_id = None;
        if state.status.take().is_some() {
            build::prepare_with(&mut state, false)
        } else {
            None
        }
    };
    build::render_prepared(request);
}

/// 宿主加载时对齐时区，避免默认日期落在 UTC 昨天
pub async fn sync_today() {
    let offset = os::timezone_offset_minutes().await;
    let today = local_date(Utc::now(), offset);
    let mut state = write_state();
    if state.today != today {
        state.today = today.clone();
        if state.event_data.name.is_empty() {
            state.event_data.time = today;
        }
    }
}

pub async fn find_device() -> Option<String> {
    let devices = device::get_connected_device_list().await;
    match devices.first() {
        Some(info) => Some(info.addr.clone()),
        None => {
            set_status_timed(StatusKind::Error, dispatch::NO_DEVICE_TEXT);
            None
        }
    }
}

/// 启动快应用并登记延时发送：真正的发送在定时器事件里进行（E-04）
pub async fn launch_then_send(payload: String, purpose: SendPurpose) -> Result<u64, SendStartError> {
    // Reserve the single-flight slot before any host await. Otherwise two rapid
    // clicks can both pass the busy check while device/app lookup is suspended.
    let token = {
        let mut state = write_state();
        if state.session_id.is_empty() {
            state.session_id = protocol::new_session_id(state.send_sequence.wrapping_add(1));
        }
        state.reserve_send(payload, purpose)
    };
    let Some(token) = token else { return Err(SendStartError::Busy) };
    set_status_timed(StatusKind::Info, "正在连接腕端…");

    let Some(device_addr) = find_device().await else {
        clear_reservation(token);
        return Err(SendStartError::Unavailable);
    };
    let connected = device::get_connected_device_list().await;
    if !connected.iter().any(|device| device.addr == device_addr) {
        clear_reservation(token);
        set_status_timed(StatusKind::Error, "连接设备已变化，请重新同步");
        return Err(SendStartError::Unavailable);
    }
    let bound = write_state().attach_send_device(token, device_addr.clone());
    if !bound {
        clear_reservation(token);
        set_status_timed(StatusKind::Error, "操作目标与当前设备或列表不一致，请重新同步后操作");
        return Err(SendStartError::Unavailable);
    }
    let Some(app) = find_app(&device_addr).await else {
        clear_reservation(token);
        return Err(SendStartError::Unavailable);
    };
    let still_connected = device::get_connected_device_list().await.iter().any(|device| device.addr == device_addr);
    if !still_connected || thirdpartyapp::launch_qa(device_addr.clone(), app, "/index".to_string()).await.is_err() {
        clear_reservation(token);
        set_status_timed(StatusKind::Error, "启动腕端应用失败");
        return Err(SendStartError::Unavailable);
    }
    let reserved = {
        let mut state = write_state();
        state.attach_send_device(token, device_addr)
    };
    if !reserved {
        return Err(SendStartError::Busy);
    }
    timer::set_timeout(APP_LAUNCH_SETTLE_MS, &format!("{}{}", SEND_TIMER, token));
    set_status_timed(StatusKind::Info, "已启动腕端应用，准备发送…");
    Ok(token)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendStartError {
    Busy,
    Unavailable,
}

async fn find_app(device_addr: &str) -> Option<thirdpartyapp::AppInfo> {
    let apps = thirdpartyapp::get_thirdparty_app_list(device_addr.to_string()).await;
    let Ok(apps) = apps else {
        set_status_timed(StatusKind::Error, "获取应用列表失败");
        return None;
    };
    let Some(app) = apps.into_iter().find(|app| app.package_name == PKG_NAME) else {
        set_status_timed(StatusKind::Error, dispatch::APP_MISSING_TEXT);
        return None;
    };
    if app.version_code < MIN_APP_VERSION_CODE {
        set_status_timed(StatusKind::Error, dispatch::APP_TOO_OLD_TEXT);
        return None;
    }
    Some(app)
}

fn clear_reservation(token: u64) {
    let request = {
        let mut state = write_state();
        if state.finish_send(token).is_some() {
            build::prepare_with(&mut state, false)
        } else {
            None
        }
    };
    build::render_prepared(request);
}

/// 定时器事件推进的发送步骤：有界重试，文案不夸大（E-08 / E-09）
pub async fn drive_pending_send(token: u64) {
    let pending = {
        let mut state = write_state();
        state.claim_send(token)
    };
    let Some(pending) = pending else { return };
    let current_devices = device::get_connected_device_list().await;
    if !current_devices.iter().any(|device| device.addr == pending.device_addr) {
        clear_reservation(token);
        set_status_timed(StatusKind::Error, "目标设备已断开，业务消息未发送，请重新连接后同步");
        return;
    }
    let needs_handshake = read_state().handshake_token != Some(token);
    if needs_handshake {
        if register::register_interconnect_recv(pending.device_addr.clone(), PKG_NAME.to_string()).await.is_err() {
            clear_reservation(token);
            set_status_timed(StatusKind::Error, "腕端消息接收器注册失败，业务消息未发送");
            return;
        }
        write_state().start_handshake(token);
        let hello = protocol::capabilities(&pending.session_id);
        if interconnect::send_qaic_message(pending.device_addr.clone(), PKG_NAME.to_string(), hello).await.is_err() {
            clear_reservation(token);
            set_status_timed(StatusKind::Error, "能力协商发送失败，业务消息未发送");
            return;
        }
    }
    let waiting = write_state().poll_handshake(token, HANDSHAKE_MAX_POLLS);
    if waiting {
        timer::set_timeout(HANDSHAKE_POLL_MS, &format!("{}{}", SEND_TIMER, token));
        set_status_timed(StatusKind::Info, "正在等待腕端能力确认…");
        return;
    }
    let current_devices = device::get_connected_device_list().await;
    let valid = {
        let state = read_state();
        state.pending_send.as_ref().is_some_and(|item| item.token == token)
            && state.target_device.as_deref() == Some(pending.device_id.as_str())
            && state.session_id == pending.session_id
            && current_devices.iter().any(|device| device.addr == pending.device_addr)
    };
    if !valid {
        clear_reservation(token);
        set_status_timed(StatusKind::Error, "握手期间目标发生变化，业务消息未发送");
        return;
    }
    let business: serde_json::Value = serde_json::from_str(&pending.payload).unwrap_or_default();
    if business.get("index").is_some() && read_state().snapshot_token.is_none() {
        clear_reservation(token);
        set_status_timed(StatusKind::Error, "旧端列表已过期，请重新同步后操作；业务消息未发送");
        return;
    }
    if business.get("id").is_some() && !read_state().supports("stableId") {
        clear_reservation(token);
        set_status_timed(StatusKind::Error, "腕端未确认稳定 ID 能力，请更新腕端后重新同步；业务消息未发送");
        return;
    }
    let (payload, result_protocol) = {
        let mut state = write_state();
        let enabled = pending.purpose == SendPurpose::Mutation
            && ["requestId", "mutationResult"].iter().all(|required| state.supports(required));
        if enabled {
            state.begin_result_wait(&pending);
            (protocol::set_request_identity_for(&pending.payload, &pending.request_id, &pending.session_id, &pending.device_id), true)
        } else if pending.purpose == SendPurpose::Fetch {
            let identified = state.supports("listRequestId");
            state.begin_list_wait(&pending, identified);
            let mut payload = if identified {
                protocol::set_request_identity_for(&pending.payload, &pending.request_id, &pending.session_id, &pending.device_id)
            } else { pending.payload.clone() };
            if identified && state.supports("listBatches") {
                let mut value: serde_json::Value = serde_json::from_str(&payload).unwrap();
                value["listBatches"] = serde_json::json!(true);
                payload = value.to_string();
            }
            (payload, false)
        } else { (pending.payload.clone(), false) }
    };
    if pending.purpose == SendPurpose::Fetch {
        timer::set_timeout(RESULT_TIMEOUT_MS, &format!("{}{}", LIST_TIMER, token));
    }
    let result =
        interconnect::send_qaic_message(pending.device_addr.clone(), PKG_NAME.to_string(), payload)
            .await;
    if result.is_ok() {
        let mut state = write_state();
        let Some(completed) = state.finish_send(token) else {
            return;
        };
        let purpose = completed.purpose;
        if purpose != SendPurpose::Fetch {
            if !result_protocol { state.require_confirmation(completed.base_revision); }
        }
        drop(state);
        if purpose == SendPurpose::Fetch {
            // Interconnect accepted the request; completion is the later list response.
            set_status_timed(StatusKind::Info, "请求已发送，等待腕端数据…");
        } else {
            if result_protocol {
                timer::set_timeout(RESULT_TIMEOUT_MS, &format!("{}{}", RESULT_TIMER, token));
                set_status_timed(StatusKind::Info, "已发送，等待腕端保存确认…");
            } else {
                set_status_timed(StatusKind::Info, dispatch::SENT_TEXT);
            }
        }
        return;
    }
    fail_attempt(token, pending, dispatch::SEND_FAILED_TEXT).await;
}

async fn fail_attempt(token: u64, pending: PendingSend, message: &str) {
    let attempts = pending.attempts + 1;
    if pending.purpose == SendPurpose::Fetch && attempts < MAX_SEND_ATTEMPTS {
        let mut state = write_state();
        if !state.retry_send(token, attempts) {
            return;
        }
        drop(state);
        timer::set_timeout(RETRY_INTERVAL_MS, &format!("{}{}", SEND_TIMER, token));
        set_status_timed(StatusKind::Info, "发送未确认，正在重试…");
        return;
    }
    {
        let mut state = write_state();
        let Some(finished) = state.finish_send(token) else {
            return;
        };
        if finished.purpose != SendPurpose::Fetch {
            if state.result_request.is_none() { state.require_confirmation(finished.base_revision); }
        }
    }
    let final_message = if pending.purpose == SendPurpose::Fetch {
        message
    } else {
        "发送结果未确认，请重新获取腕端数据核实；不要直接重复新增"
    };
    set_status_timed(StatusKind::Error, final_message);
    if read_state().result_deadline == Some(token) {
        timer::set_timeout(RESULT_TIMEOUT_MS, &format!("{}{}", RESULT_TIMER, token));
    }
}

/// 用户刷新时也核实原请求，不重发 mutation。
pub async fn query_original_result() {
    let original = read_state().result_request.clone();
    let Some(original) = original else { return; };
    let devices = device::get_connected_device_list().await;
    if !devices.iter().any(|device| device.addr == original.device_id) { return; }
    let request = protocol::request_status_for(&original.request_id, &original.session_id, &original.device_id);
    let _ = interconnect::send_qaic_message(original.device_id, PKG_NAME.to_string(), request).await;
}

/// 删除二次确认：对话框显示名称 + 日期，避免同名误删（E-19）
pub async fn confirm_delete(event: &crate::ui::state::EventData) -> bool {
    let info = dialog::DialogInfo {
        title: dispatch::CONFIRM_DELETE_TITLE.to_string(),
        content: format!("确认删除「{}」？该操作会同时影响腕端表盘文件。", event.label()),
        buttons: vec![
            dialog::DialogButton {
                id: dispatch::CONFIRM_DELETE_CANCEL.to_string(),
                primary: false,
                content: "取消".to_string(),
            },
            dialog::DialogButton {
                id: dispatch::CONFIRM_DELETE_ACCEPT.to_string(),
                primary: true,
                content: "删除".to_string(),
            },
        ],
    };
    let result = dialog::show_dialog(dialog::DialogType::Alert, dialog::DialogStyle::System, info).await;
    result.clicked_btn_id == dispatch::CONFIRM_DELETE_ACCEPT
}

/// 宿主定时器事件分派
pub async fn handle_timer_event(payload: &str) {
    let payload = unwrap_timer_payload(payload);
    if let Some(token) = payload.strip_prefix(SNAPSHOT_TIMER).and_then(|value| value.parse::<u64>().ok()) {
        write_state().expire_snapshot(token);
        return;
    }
    if let Some(token) = payload.strip_prefix(LIST_TIMER).and_then(|value| value.parse::<u64>().ok()) {
        let render = {
            let mut state = write_state();
            if state.expire_list(token) { build::prepare_with(&mut state, false) } else { None }
        };
        build::render_prepared(render);
        return;
    }
    if payload == HIDE_MESSAGE_TIMER {
        hide_message();
        return;
    }
    if let Some(token) = payload.strip_prefix(SEND_TIMER).and_then(|value| value.parse::<u64>().ok()) {
        drive_pending_send(token).await;
        return;
    }
    if let Some(token) = payload.strip_prefix(RESULT_TIMER).and_then(|value| value.parse::<u64>().ok()) {
        let request = {
            let state = read_state();
            if state.result_deadline != Some(token) { return; }
            if !state.protocol_v2 { return; }
            let Some(original) = state.result_request.as_ref() else { return; };
            if state.target_device.as_deref() != Some(original.device_id.as_str()) { return; }
            protocol::request_status_for(&original.request_id, &original.session_id, &original.device_id)
        };
        let device_addr = read_state().target_device.clone();
        if let Some(device_addr) = device_addr {
            let render = {
                let mut state = write_state();
                if state.result_deadline != Some(token) { return; }
                state.result_deadline = None;
                state.status = Some(crate::ui::state::StatusMessage::info("保存结果待核实，已查询原请求；迟到结果仍会接收"));
                build::prepare_with(&mut state, false)
            };
            build::render_prepared(render);
            let _ = interconnect::send_qaic_message(device_addr, PKG_NAME.to_string(), request).await;
        }
    }
}

fn unwrap_timer_payload(payload: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) {
        if let Some(text) = value.get("payloadText").and_then(|value| value.as_str()) {
            return text.to_string();
        }
        if let Some(text) = value.get("payload").and_then(|value| value.as_str()) {
            return text.to_string();
        }
    }
    payload.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer载荷兼容宿主原文与JSON包装() {
        assert_eq!(unwrap_timer_payload("send_pending:42"), "send_pending:42");
        assert_eq!(
            unwrap_timer_payload(r#"{"payloadText":"send_pending:42"}"#),
            "send_pending:42"
        );
        assert_eq!(
            unwrap_timer_payload(r#"{"payload":"hide_message"}"#),
            "hide_message"
        );
    }

    #[test]
    fn timer_token必须匹配且前缀不吞掉其他事件() {
        assert_eq!(SEND_TIMER, "send_pending:");
        assert_eq!("send_pending:42".strip_prefix(SEND_TIMER), Some("42"));
        assert_eq!("send_pending_old".strip_prefix(SEND_TIMER), None);
        assert_eq!("send_pending:42".strip_prefix(SEND_TIMER).unwrap().parse::<u64>(), Ok(42));
        assert!("send_pending:not-a-token".strip_prefix(SEND_TIMER).unwrap().parse::<u64>().is_err());
    }
}
