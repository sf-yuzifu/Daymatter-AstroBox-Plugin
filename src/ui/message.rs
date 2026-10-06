//! 宿主交互：状态条、定时器、设备与快应用通道（E-04 / E-17 / E-19）。
//!
//! 等待一律由宿主定时器事件推进：`timer.set_timeout` 只注册并立即返回，
//! 插件里没有 async sleep，因此「启动快应用 → 再发消息」拆成启动 + 定时器两步，
//! 取代旧实现的 `thread::sleep(2 秒)`（会阻塞整个 guest，界面无响应）。

use crate::astrobox::psys_host_v4::{device, dialog, interconnect, os, register, thirdpartyapp, timer};
use crate::ui::build;
use crate::ui::dispatch;
use crate::ui::protocol::{MIN_APP_VERSION_CODE, PKG_NAME};
use crate::ui::state::{local_date, write_state, PendingSend, SendPurpose, StatusKind};
use chrono::Utc;

pub const HIDE_MESSAGE_TIMER: &str = "hide_message";
pub const SEND_TIMER: &str = "send_pending:";
/// 启动快应用后等待其完成加载，再发送互联消息
pub const APP_LAUNCH_SETTLE_MS: u64 = 1500;
pub const MAX_SEND_ATTEMPTS: u32 = 3;
pub const RETRY_INTERVAL_MS: u64 = 800;

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
        state.reserve_send(payload, purpose)
    };
    let Some(token) = token else { return Err(SendStartError::Busy) };
    set_status_timed(StatusKind::Info, "正在连接腕端…");

    let Some(device_addr) = find_device().await else {
        clear_reservation(token);
        return Err(SendStartError::Unavailable);
    };
    let Some(app) = find_app(&device_addr).await else {
        clear_reservation(token);
        return Err(SendStartError::Unavailable);
    };
    if thirdpartyapp::launch_qa(device_addr.clone(), app, "/index".to_string()).await.is_err() {
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
    if register::register_interconnect_recv(pending.device_addr.clone(), PKG_NAME.to_string()).await.is_err() {
        fail_attempt(token, pending, "腕端消息接收器注册失败").await;
        return;
    }
    let result =
        interconnect::send_qaic_message(pending.device_addr.clone(), PKG_NAME.to_string(), pending.payload.clone())
            .await;
    if result.is_ok() {
        let mut state = write_state();
        let Some(completed) = state.finish_send(token) else {
            return;
        };
        let purpose = completed.purpose;
        if purpose != SendPurpose::Fetch {
            state.require_confirmation(completed.base_revision);
        }
        drop(state);
        if purpose == SendPurpose::Fetch {
            // Interconnect accepted the request; completion is the later list response.
            set_status_timed(StatusKind::Info, "请求已发送，等待腕端数据…");
        } else {
            set_status_timed(StatusKind::Success, dispatch::SENT_TEXT);
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
            state.require_confirmation(finished.base_revision);
        }
    }
    let final_message = if pending.purpose == SendPurpose::Fetch {
        message
    } else {
        "发送结果未确认，请重新获取腕端数据核实；不要直接重复新增"
    };
    set_status_timed(StatusKind::Error, final_message);
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
    if payload == HIDE_MESSAGE_TIMER {
        hide_message();
        return;
    }
    if let Some(token) = payload.strip_prefix(SEND_TIMER).and_then(|value| value.parse::<u64>().ok()) {
        drive_pending_send(token).await;
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
