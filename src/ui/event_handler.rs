//! 宿主事件入口：UI 事件 → 表驱动派发 → 状态变换 → 按需重建；互联消息与定时器分别落地。

use base64::{engine::general_purpose::STANDARD, Engine as _};

use crate::astrobox::psys_host_v4::dialog::{pick_file, FilterConfig, PickConfig};
use crate::ui::build;
use crate::ui::dispatch::{self, Action, UiEventKind};
use crate::ui::message::{self, SendStartError};
use crate::ui::protocol::{self, EventRef};
use crate::ui::state::{read_state, write_state, EventData, FormScope, SendPurpose, StatusKind};
use crate::ui::validation::{validate_date_format, validate_name, normalized_name};

fn map_event_kind(event: &ui::Event) -> Option<UiEventKind> {
    match event {
        ui::Event::Click => Some(UiEventKind::Click),
        ui::Event::Change => Some(UiEventKind::Change),
        _ => None,
    }
}

use crate::astrobox::psys_host_v4::{ui, timer};

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
        Action::ExportBackup => backup_file(false).await,
        Action::ImportBackup => backup_file(true).await,
        Action::RestoreBackup => {
            let payload = { let state = read_state(); if state.busy() { return; } state.restore_payload() };
            match payload {
                Ok(payload) => { let _ = message::launch_then_send(payload, SendPurpose::Mutation).await; }
                Err(error) => message::set_status(StatusKind::Error, error),
            }
        }
        Action::Primary(cancel) => {
            let payload = {
                let state = read_state();
                if state.busy() { return; }
                state.primary_payload(cancel)
            };
            match payload {
                Ok(payload) => { let _ = message::launch_then_send(payload, SendPurpose::Mutation).await; }
                Err(error) => message::set_status(StatusKind::Error, error),
            }
        }
        Action::MoveEvent(direction) => {
            let key = read_state().selected_event_key.clone();
            if let Some(id) = key.as_deref().and_then(|k| k.strip_prefix("id:")) {
                let payload = serde_json::json!({"type":"moveEvent", "id":id, "direction":direction}).to_string();
                let _ = message::launch_then_send(payload, SendPurpose::Mutation).await;
            } else { message::set_status(StatusKind::Error, "移动需要稳定 ID，请更新腕端并同步"); }
        }
        Action::SortMode => {
            // 不从临时 RwLockReadGuard 借值到发送作用域；发送入口立即获取写锁。
            let mode = {
                let state = read_state();
                state.next_sort_mode()
            };
            let _ = message::launch_then_send(serde_json::json!({"type":"setSortMode", "sortMode":mode}).to_string(), SendPurpose::Mutation).await;
        }
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
        Action::SwitchSegment(super::state::EventType::AddBackground) => {
            if !read_state().busy() { let _ = message::launch_then_send(protocol::get_all_events(), SendPurpose::Fetch).await; }
        }
        Action::SendBackground => {
            let devices = crate::astrobox::psys_host_v4::device::get_connected_device_list().await;
            let matches = { let state = read_state(); devices.first().is_some_and(|d| state.background_device.as_deref() == Some(d.addr.as_str())) };
            if !matches { message::set_status(StatusKind::Error, "连接设备已变化，请重新读取屏幕并生成预览"); return; }
            let data = { let state = read_state(); if state.busy() { return; } state.background_preview.clone() };
            if let Some(data) = data {
                match STANDARD.decode(&data) {
                    Ok(bytes) => {
                        let hash = super::background::checksum(&bytes);
                        let payload = serde_json::json!({"type":"beginBG","bytes":bytes.len(),"checksum":hash}).to_string();
                        { let mut state = write_state(); state.background_bytes = bytes; state.background_offset = 0; state.background_frame = None; }
                        let _ = message::launch_then_send(payload, SendPurpose::Background).await;
                    }
                    Err(_) => message::set_status(StatusKind::Error, "预览成品损坏，请重新选图"),
                }
            } else { message::set_status(StatusKind::Error, "参数已变化，请重新选择图片生成预览"); }
        }
        _ => {}
    }
}

async fn backup_file(import: bool) {
    use crate::astrobox::psys_host_v4::dialog;
    let backup = {
        let mut state = write_state();
        if state.busy() { return; }
        match state.export_backup() {
            Ok(backup) => { state.local_file_busy = true; state.result_status = None; Ok(backup) }
            Err(error) => Err(error),
        }
    };
    let backup = match backup { Ok(value) => value, Err(error) => { message::set_status(StatusKind::Error, error); return; } };
    build::refresh(false);
    let filter = FilterConfig {multiple:false, extensions:vec!["json".into()], default_directory:String::new(), default_file_name:"daymatter-backup.json".into()};
    let result: Result<(), String> = if import {
        match pick_file(PickConfig {read:true, copy_to:None}, filter).await {
            Ok(file) => {
                if file.data.len() > super::backup::MAX_BYTES { Err("备份超过 240 KiB 预算".into()) }
                else {
                    match serde_json::from_slice(&file.data) {
                        Ok(value) => write_state().preview_backup(value).map_err(str::to_owned),
                        Err(_) => Err("备份不是合法 UTF-8 JSON".into()),
                    }
                }
            }
            Err(error) => Err(format!("选择备份未完成：{error}")),
        }
    } else {
        match dialog::save_file_start(filter).await {
            Ok(session) => {
                let bytes = serde_json::to_vec(&backup).unwrap();
                let mut result = Ok(());
                for chunk in bytes.chunks(16 * 1024) {
                    if let Err(error) = dialog::save_file_write_chunk(session.session_id, chunk.to_vec()).await { result = Err(error); break; }
                }
                if result.is_ok() { result = dialog::save_file_finish(session.session_id).await; }
                if result.is_err() { dialog::save_file_abort(session.session_id).await; }
                result
            }
            Err(error) => Err(format!("保存备份未完成：{error}")),
        }
    };
    write_state().local_file_busy = false;
    match result {
        Ok(()) => message::set_status(StatusKind::Success, if import {"备份已校验，请查看预览并确认恢复规则"} else {"完整 JSON 备份已保存"}),
        Err(error) => message::set_status(StatusKind::Error, &error),
    }
}

fn draft_of(scope: FormScope) -> EventData {
    read_state().draft(scope).cloned().unwrap_or_default()
}

fn validate_draft(draft: &EventData, previous: Option<&str>) -> Option<String> {
    if let Err(error) = protocol::validate_extensions(&serde_json::json!(draft.extensions)) { return Some(error.into()); }
    for (field, allowed) in [("repeat", &["none", "yearly"][..]), ("displayUnit", &["days", "weeks", "months", "years"][..])] {
        if let Some(value) = draft.extensions.get(field) {
            if !value.as_str().is_some_and(|text| allowed.contains(&text)) { return Some("重复规则或显示单位无效，请按提示选择".into()); }
        }
    }
    if let Err(error) = validate_name(&draft.name, previous) { return Some(error.to_string()); }
    validate_date_format(draft.time.trim()).err().map(|error| error.to_string())
}

async fn submit_add() {
    let mut draft = draft_of(FormScope::Add);
    draft = match super::lunar::prepare(&draft) {Ok(draft)=>draft,Err(error)=>{message::set_status(StatusKind::Error,error);return;}};
    draft.time = draft.time.trim().to_string();
    if let Some(error) = validate_draft(&draft, None) {
        message::set_status_timed(StatusKind::Error, &error);
        return;
    }
    draft.name = normalized_name(&draft.name, None);
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
    draft = match super::lunar::prepare(&draft) {Ok(draft)=>draft,Err(error)=>{message::set_status(StatusKind::Error,error);return;}};
    let previous = {
        let state = read_state();
        state.index_of_key(&key).and_then(|index| state.all_events.get(index)).map(|event| event.name.clone())
    };
    draft.time = draft.time.trim().to_string();
    if let Some(error) = validate_draft(&draft, previous.as_deref()) {
        message::set_status_timed(StatusKind::Error, &error);
        return;
    }
    draft.name = normalized_name(&draft.name, previous.as_deref());
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
    let devices = crate::astrobox::psys_host_v4::device::get_connected_device_list().await;
    let target = {
        let mut state = write_state();
        if state.busy() { return; }
        if !devices.first().is_some_and(|d| state.background_device.as_deref() == Some(d.addr.as_str())) {
            drop(state);
            message::set_status(StatusKind::Error, "请先读取连接设备屏幕；更新腕端后再试");
            return;
        }
        let dimensions = state.background_width.parse::<u32>().ok().zip(state.background_height.parse::<u32>().ok());
        let Some((width, height)) = dimensions.filter(|(w,h)| (1..=2048).contains(w) && (1..=2048).contains(h)) else {
            drop(state);
            message::set_status(StatusKind::Error, "目标宽高须为1～2048px");
            return;
        };
        state.local_file_busy = true;
        (width, height, state.background_cover)
    };
    build::refresh(false);
    let pick_config = PickConfig { read: true, copy_to: None };
    let filter_config = FilterConfig {
        multiple: false,
        extensions: vec!["png".to_string(), "jpg".to_string(), "jpeg".to_string()],
        default_directory: String::new(),
        default_file_name: String::new(),
    };
    let picked = pick_file(pick_config, filter_config).await;
    let Ok(picked) = picked else {
        write_state().local_file_busy = false;
        message::set_status_timed(StatusKind::Error, "未选择图片");
        return;
    };
    let compressed = crate::ui::image::prepare_for_screen(&picked.data, target.0, target.1, target.2);
    // 不把原始大图保留到连接/发送await期间。
    drop(picked);
    write_state().local_file_busy = false;
    match compressed {
        Ok(compressed) => {
            {
                let mut state = write_state();
                state.background_preview = Some(STANDARD.encode(&compressed));
                state.background_detail = format!("原尺寸{}×{} · {} · JPEG {}B；3KiB小分片逐片落盘，确认后才发送。", target.0, target.1,
                    if target.2 {"居中裁剪"} else {"等比留黑边"}, compressed.len());
            }
            message::set_status(StatusKind::Success, "成品已生成，请核对预览后发送");
        }
        Err(error) => {
            message::set_status_timed(StatusKind::Error, &error);
        }
    }
}

/// 腕端互联消息：解析事件列表并刷新界面（E-12 无效条目与越界防护）
pub async fn handle_interconnect_message(payload: &str) {
    if let Ok(value) = protocol::decode_message(payload) {
        if value["type"] == "backgroundAck" { message::background_ack(&value).await; return; }
        if matches!(value.get("type").and_then(|v| v.as_str()), Some("eventListBatch" | "eventListError")) {
            let (render, token) = {
                let mut state = write_state();
                if state.accept_list_batch(&value) {
                    (build::prepare_with(&mut state, true), state.snapshot_token)
                } else { (None, None) }
            };
            if let Some(token) = token {
                timer::set_timeout(message::LEGACY_SNAPSHOT_MS, &format!("{}{}", message::SNAPSHOT_TIMER, token));
            }
            build::render_prepared(render);
            return;
        }
        if value.get("type").and_then(|v| v.as_str()) == Some("capabilities") {
            let mut state = write_state();
            state.accept_capabilities(&value);
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
            let mut accepted_token = None;
            let request = {
                let mut state = write_state();
                // 已请求分批时，不允许同身份整包绕过齐全校验。
                if state.list_request.as_ref().is_some_and(|request| request.batched) { return; }
                if state.accept_event_list(parsed) {
                    accepted_token = state.snapshot_token;
                    build::prepare_with(&mut state, true)
                } else { None }
            };
            if let Some(token) = accepted_token {
                timer::set_timeout(message::LEGACY_SNAPSHOT_MS, &format!("{}{}", message::SNAPSHOT_TIMER, token));
            }
            build::render_prepared(request);
        }
        Err(issue) => {
            // 无可信来源的畸形回包不结算请求，统一由有界列表超时收尾。
            tracing::debug!("ignored malformed list: {}", issue.message());
        }
    }
}

pub async fn handle_timer_event(payload: &str) {
    message::handle_timer_event(payload).await;
}
