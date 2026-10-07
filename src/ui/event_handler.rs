//! 宿主事件入口：UI 事件 → 表驱动派发 → 状态变换 → 按需重建；互联消息与定时器分别落地。

use base64::{engine::general_purpose::STANDARD, Engine as _};

use crate::astrobox::psys_host_v4::dialog::{pick_file, FilterConfig, PickConfig};
use crate::ui::build;
use crate::ui::dispatch::{self, Action, UiEventKind};
use crate::ui::message::{self, SendStartError};
use crate::ui::protocol::{self, EventRef};
use crate::ui::state::{read_state, write_state, EventData, FormScope, SendPurpose, StatusKind, StatusMessage};
use crate::ui::validation::validate_date_format;

fn map_event_kind(event: &ui::Event) -> Option<UiEventKind> {
    match event {
        ui::Event::Click => Some(UiEventKind::Click),
        ui::Event::Change => Some(UiEventKind::Change),
        _ => None,
    }
}

use crate::astrobox::psys_host_v4::ui;

pub async fn ui_event_processor(event_type: ui::Event, event_id: &str, event_payload: &str) {
    tracing::info!("UI 事件: id={}, type={:?}", event_id, event_type);
    let Some(kind) = map_event_kind(&event_type) else {
        return;
    };
    let Some(action) = dispatch::resolve(event_id, kind, event_payload) else {
        return;
    };
    run_action(action).await;
}

async fn run_action(action: Action) {
    let rebuild = {
        let mut state = write_state();
        dispatch::apply_action(&mut state, action.clone()).rebuild
    };
    if rebuild {
        build::refresh(false);
    }

    match action {
        Action::SubmitAdd => submit_add().await,
        Action::SubmitModify => submit_modify().await,
        Action::FetchEvents => {
            message::query_original_result().await;
            if message::launch_then_send(protocol::get_all_events(), SendPurpose::Fetch).await
                == Err(SendStartError::Busy)
            {
                message::set_status_timed(StatusKind::Info, "已有操作正在发送，请稍候");
            }
        }
        Action::RequestDelete => request_delete().await,
        Action::PickBackground => pick_background().await,
        _ => {}
    }
}

fn draft_of(scope: FormScope) -> EventData {
    read_state().draft(scope).cloned().unwrap_or_default()
}

fn validate_draft(draft: &EventData) -> Option<String> {
    if draft.name.trim().is_empty() {
        return Some(dispatch::NAME_REQUIRED_TEXT.to_string());
    }
    if draft.name.trim().chars().count() > 50 {
        return Some("事件名称最多 50 个字符".to_string());
    }
    validate_date_format(draft.time.trim()).err().map(|error| error.to_string())
}

async fn submit_add() {
    let mut draft = draft_of(FormScope::Add);
    draft.name = draft.name.trim().to_string();
    draft.time = draft.time.trim().to_string();
    if let Some(error) = validate_draft(&draft) {
        message::set_status_timed(StatusKind::Error, &error);
        return;
    }
    if message::launch_then_send(protocol::add_event(&draft), SendPurpose::Mutation).await
        == Err(SendStartError::Busy)
    {
        message::set_status_timed(StatusKind::Info, "已有操作正在进行，请先同步腕端结果");
    }
}

async fn submit_modify() {
    let key = read_state().selected_event_key.clone();
    let Some(key) = key else {
        message::set_status_timed(StatusKind::Error, dispatch::NO_SELECTION_TEXT);
        return;
    };
    if read_state().index_of_key(&key).is_none() {
        message::set_status_timed(StatusKind::Error, "事件列表已变化，请重新获取后选择");
        return;
    }
    let mut draft = draft_of(FormScope::Modify);
    draft.name = draft.name.trim().to_string();
    draft.time = draft.time.trim().to_string();
    if let Some(error) = validate_draft(&draft) {
        message::set_status_timed(StatusKind::Error, &error);
        return;
    }
    let target = event_ref_for(&key);
    match message::launch_then_send(protocol::change_event(&draft, &target), SendPurpose::Mutation).await {
        Ok(_) => {
            let mut state = write_state();
            // 仅在命令进入发送流程后返回概览；腕端落盘结果需后续确认。
            state.back_to_overview();
            drop(state);
            build::refresh(true);
        }
        Err(SendStartError::Busy) => message::set_status_timed(StatusKind::Info, "已有操作正在进行，请先同步腕端结果"),
        Err(SendStartError::Unavailable) => {}
    }
}

/// 选择键 → 协议定位：带 ID 用 ID，旧数据用下标（E-11）
fn event_ref_for(key: &str) -> EventRef {
    if let Some(id) = key.strip_prefix("id:") {
        return EventRef::Id(id.to_string());
    }
    match key.strip_prefix("idx:").and_then(|text| text.parse::<usize>().ok()) {
        Some(index) => EventRef::Index(index),
        None => EventRef::Id(key.to_string()),
    }
}

async fn request_delete() {
    let (key, event) = {
        let mut state = write_state();
        if state.delete_dialog_open || state.busy() {
            return;
        }
        let key = state.selected_event_key.clone();
        let event = key
            .as_deref()
            .and_then(|key| state.index_of_key(key))
            .and_then(|index| state.all_events.get(index).cloned());
        if event.is_some() {
            state.delete_dialog_open = true;
        }
        (key, event)
    };
    let (Some(key), Some(event)) = (key, event) else {
        message::set_status_timed(StatusKind::Error, dispatch::NO_SELECTION_TEXT);
        return;
    };
    let confirmed = message::confirm_delete(&event).await;
    {
        let mut state = write_state();
        state.delete_dialog_open = false;
    }
    build::refresh(false);
    if !confirmed {
        message::set_status(StatusKind::Info, "已取消删除");
        return;
    }
    let target_still_matches = {
        let state = read_state();
        state.selected_event_key.as_deref() == Some(key.as_str())
            && state
                .index_of_key(&key)
                .and_then(|index| state.all_events.get(index))
                .is_some_and(|current| {
                    current.id == event.id && current.name == event.name && current.time == event.time
                })
    };
    if !target_still_matches {
        message::set_status_timed(StatusKind::Error, "腕端列表已变化，请重新获取并确认删除");
        return;
    }
    // 不乐观删除：插件发出命令不等于腕端已落盘。等腕端回传列表确认后再移除。
    match message::launch_then_send(protocol::delete_event(&event_ref_for(&key)), SendPurpose::Mutation).await {
        Ok(_) | Err(SendStartError::Unavailable) => {}
        Err(SendStartError::Busy) => message::set_status_timed(StatusKind::Info, "已有操作正在进行，请先同步腕端结果"),
    }
}

async fn pick_background() {
    let pick_config = PickConfig { read: true, copy_to: None };
    let filter_config = FilterConfig {
        multiple: false,
        extensions: vec!["png".to_string(), "jpg".to_string(), "jpeg".to_string()],
        default_directory: String::new(),
        default_file_name: String::new(),
    };
    let picked = pick_file(pick_config, filter_config).await;
    let Ok(picked) = picked else {
        message::set_status_timed(StatusKind::Error, "未选择图片");
        return;
    };
    match crate::ui::image::compress_for_watch(&picked.data) {
        Ok(compressed) => {
            if message::launch_then_send(
                protocol::add_background(&STANDARD.encode(&compressed)),
                SendPurpose::Background,
            )
            .await == Err(SendStartError::Busy)
            {
                message::set_status_timed(StatusKind::Info, "已有操作正在进行，请先同步腕端结果");
            }
        }
        Err(error) => {
            message::set_status_timed(StatusKind::Error, &error);
        }
    }
}

/// 腕端互联消息：解析事件列表并刷新界面（E-12 无效条目与越界防护）
pub async fn handle_interconnect_message(payload: &str) {
    if let Ok(value) = protocol::decode_message(payload) {
        if value.get("type").and_then(|v| v.as_str()) == Some("capabilities") {
            let mut state = write_state();
            let session = value.get("sessionId").and_then(|v| v.as_str()).unwrap_or("");
            if session == state.session_id {
                state.protocol_v2 = value.get("protocolVersion").and_then(|v| v.as_u64()).unwrap_or(0) >= 2;
                state.capabilities = value.get("capabilities").and_then(|v| v.as_array()).map(|items|
                    items.iter().filter_map(|item| item.as_str().map(str::to_owned)).collect()).unwrap_or_default();
            }
            return;
        }
        if matches!(value.get("type").and_then(|v| v.as_str()), Some("mutationResult" | "requestStatus")) {
            let request = {
                let mut state = write_state();
                if state.apply_request_message(&value) { build::prepare_with(&mut state, false) } else { None }
            };
            build::render_prepared(request);
            return;
        }
    }
    match protocol::parse_event_list(payload) {
        Ok(parsed) => {
            let skipped = parsed.skipped;
            let request = {
                let mut state = write_state();
                let envelope: serde_json::Value = serde_json::from_str(payload).unwrap_or_default();
                let envelope = envelope.get("data").unwrap_or(&envelope);
                if let Some(session) = envelope.get("sessionId").and_then(|v| v.as_str()) {
                    if session != state.session_id { return; }
                }
                if state.target_device.is_none() { return; }
                state.all_events = parsed.events;
                state.has_fetched_events = true;
                state.event_page = 0;
                state.apply_confirmed_list(parsed.primary_id, parsed.revision);
                if let Some(key) = state.selected_event_key.clone() {
                    if state.index_of_key(&key).is_none() {
                        state.clear_selection();
                    }
                }
                let text = if skipped > 0 {
                    format!("{}（{} 条数据无效已跳过）", dispatch::FETCH_SUCCESS_TEXT, skipped)
                } else {
                    dispatch::FETCH_SUCCESS_TEXT.to_string()
                };
                state.status = Some(if state.result_request.is_some() {
                    StatusMessage::info("列表已刷新，原请求结果仍待核实；不要重复新增")
                } else { StatusMessage::success(&text) });
                build::prepare_with(&mut state, true)
            };
            build::render_prepared(request);
        }
        Err(issue) => {
            message::set_status_timed(StatusKind::Error, issue.message());
        }
    }
}

pub async fn handle_timer_event(payload: &str) {
    message::handle_timer_event(payload).await;
}
