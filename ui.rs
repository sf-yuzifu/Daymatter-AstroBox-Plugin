use crate::astrobox::psys_host::{
    self, device, dialog, interconnect, register, thirdpartyapp, timer, ui,
};
use serde_json::Value;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

const PACKAGE_NAME: &str = "com.bandbbs.ebook";
const HANDSHAKE_TAG: &str = "__hs__";
const FILE_TAG: &str = "file";
const FILE_CHUNK_BYTES: usize = 48 * 1024;

const EVENT_PICK_FILE: &str = "pick_file";
const EVENT_SEND_FILE: &str = "send_file";
const EVENT_CANCEL_SEND: &str = "cancel_send";

const TIMER_HIDE_MESSAGE: &str = "timer_hide_message";
const TIMER_START_HANDSHAKE: &str = "timer_start_handshake";
const TIMER_HANDSHAKE_TIMEOUT: &str = "timer_handshake_timeout";

#[derive(Clone, Copy)]
enum StatusTone {
    Neutral,
    Success,
    Error,
}

struct TransferState {
    device_addr: String,
    file_name: String,
    text: String,
    total_chunks: usize,
    boundaries: Vec<usize>,
    current_chunk: usize,
    last_chunk_time: Option<SystemTime>,
    pending_start: bool,
    handshake_complete: bool,
}

struct UiState {
    root_element_id: Option<String>,
    file_name: Option<String>,
    file_size_bytes: usize,
    file_text: Option<String>,
    progress: f32,
    speed_text: Option<String>,
    status_message: Option<String>,
    status_tone: StatusTone,
    is_sending: bool,
    transfer: Option<TransferState>,
    hide_message_timer_id: Option<u64>,
    handshake_timer_id: Option<u64>,
}

static UI_STATE: OnceLock<Mutex<UiState>> = OnceLock::new();

fn ui_state() -> &'static Mutex<UiState> {
    UI_STATE.get_or_init(|| {
        Mutex::new(UiState {
            root_element_id: None,
            file_name: None,
            file_size_bytes: 0,
            file_text: None,
            progress: 0.0,
            speed_text: None,
            status_message: None,
            status_tone: StatusTone::Neutral,
            is_sending: false,
            transfer: None,
            hide_message_timer_id: None,
            handshake_timer_id: None,
        })
    })
}

pub fn ui_event_processor(evtype: ui::Event, event: &str, _event_payload: &str) {
    match evtype {
        ui::Event::Click => match event {
            EVENT_PICK_FILE => handle_pick_file(),
            EVENT_SEND_FILE => handle_send_file(),
            EVENT_CANCEL_SEND => handle_cancel_send(),
            _ => {}
        },
        _ => {}
    }
}

pub fn handle_timer_event(event_payload: &str) {
    let payload = extract_payload_text(event_payload);
    match payload.as_str() {
        TIMER_HIDE_MESSAGE => {
            let should_render = {
                let mut state = ui_state()
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.status_message = None;
                state.status_tone = StatusTone::Neutral;
                state.hide_message_timer_id = None;
                state.root_element_id.is_some()
            };
            if should_render {
                render_from_state();
            }
        }
        TIMER_START_HANDSHAKE => {
            let device_addr = {
                let mut state = ui_state().lock().unwrap_or_else(|p| p.into_inner());
                state.handshake_timer_id = None;

                let Some(t) = state.transfer.as_ref() else {
                    return;
                };
                t.device_addr.clone()
            };

            {
                let mut state = ui_state().lock().unwrap_or_else(|p| p.into_inner());
                schedule_handshake_timeout(&mut state);
            }

            send_handshake_message(&device_addr, 0);
        }
        TIMER_HANDSHAKE_TIMEOUT => {
            let mut should_render = false;
            {
                let mut state = ui_state()
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.handshake_timer_id = None;
                if state.is_sending {
                    set_status_message(
                        &mut state,
                        "连接超时，请确认手环端应用已打开",
                        StatusTone::Error,
                        true,
                    );
                    finish_transfer(&mut state, true);
                    should_render = true;
                }
            }
            if should_render {
                render_from_state();
            }
        }
        _ => {}
    }
}

pub fn handle_interconnect_message(event_payload: &str) {
    let payload = extract_payload_text(event_payload);
    let Ok(message) = serde_json::from_str::<Value>(&payload) else {
        tracing::warn!("无法解析互联消息: {}", payload);
        return;
    };

    let tag = message.get("tag").and_then(|v| v.as_str()).unwrap_or("");
    match tag {
        HANDSHAKE_TAG => handle_handshake_message(&message),
        FILE_TAG => handle_file_message(&message),
        _ => {}
    }
}

fn handle_pick_file() {
    let pick_result = wit_bindgen::block_on(async {
        let pick_config = dialog::PickConfig {
            read: true,
            copy_to: None,
        };
        let filter_config = dialog::FilterConfig {
            multiple: false,
            extensions: vec![],
            default_directory: "".to_string(),
            default_file_name: "".to_string(),
        };
        dialog::pick_file(&pick_config, &filter_config).await
    });

    let name = pick_result.name;
    let bytes = pick_result.data;
    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => {
            {
                let mut state = ui_state()
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                set_status_message(
                    &mut state,
                    "文件不是 UTF-8 文本，无法发送",
                    StatusTone::Error,
                    true,
                );
                state.file_name = None;
                state.file_text = None;
                state.file_size_bytes = 0;
                state.progress = 0.0;
                state.speed_text = None;
            }
            render_from_state();
            return;
        }
    };

    {
        let mut state = ui_state()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.is_sending {
            set_status_message(
                &mut state,
                "正在发送中，无法更换文件",
                StatusTone::Error,
                true,
            );
            drop(state);
            render_from_state();
            return;
        }
        state.file_size_bytes = text.as_bytes().len();
        state.file_name = Some(name);
        state.file_text = Some(text);
        state.progress = 0.0;
        state.speed_text = None;
        state.status_message = None;
        state.status_tone = StatusTone::Neutral;
        clear_transfer_state(&mut state, true);
    }
    render_from_state();
}

fn handle_send_file() {
    let (file_name, file_text, file_size) = {
        let state = ui_state()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.is_sending {
            return;
        }
        match (&state.file_name, &state.file_text) {
            (Some(name), Some(text)) => (name.clone(), text.clone(), state.file_size_bytes),
            _ => {
                drop(state);
                {
                    let mut state = ui_state()
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    set_status_message(&mut state, "请先选择文件", StatusTone::Error, true);
                }
                render_from_state();
                return;
            }
        }
    };

    {
        let mut state = ui_state()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.is_sending = true;
        state.progress = 0.0;
        state.speed_text = None;
        set_status_message(&mut state, "准备发送中...", StatusTone::Neutral, false);
    }
    render_from_state();

    let device_addr = match get_device_addr() {
        Ok(addr) => addr,
        Err(msg) => {
            {
                let mut state = ui_state()
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                set_status_message(&mut state, &msg, StatusTone::Error, true);
                finish_transfer(&mut state, true);
            }
            render_from_state();
            return;
        }
    };

    let app = match get_app_info(&device_addr) {
        Ok(app) => app,
        Err(msg) => {
            {
                let mut state = ui_state()
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                set_status_message(&mut state, &msg, StatusTone::Error, true);
                finish_transfer(&mut state, true);
            }
            render_from_state();
            return;
        }
    };

    let launch_ok = wit_bindgen::block_on(async {
        thirdpartyapp::launch_qa(&device_addr, &app, "/index").await
    })
    .is_ok();

    if !launch_ok {
        {
            let mut state = ui_state()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            set_status_message(&mut state, "启动应用失败", StatusTone::Error, true);
            finish_transfer(&mut state, true);
        }
        render_from_state();
        return;
    }

    let _ = wit_bindgen::block_on(async {
        register::register_interconnect_recv(&device_addr, PACKAGE_NAME).await
    });

    let transfer = match build_transfer_state(device_addr.clone(), file_name, file_text, file_size)
    {
        Ok(transfer) => transfer,
        Err(msg) => {
            {
                let mut state = ui_state()
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                set_status_message(&mut state, &msg, StatusTone::Error, true);
                finish_transfer(&mut state, true);
            }
            render_from_state();
            return;
        }
    };

    {
        let mut state = ui_state()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.transfer = Some(transfer);

        let timer_id =
            wit_bindgen::block_on(async { timer::set_timeout(1500, TIMER_START_HANDSHAKE).await });
        state.handshake_timer_id = Some(timer_id);
    }

    render_from_state();
}

fn handle_cancel_send() {
    let device_addr = {
        let state = ui_state()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !state.is_sending {
            return;
        }
        state
            .transfer
            .as_ref()
            .map(|transfer| transfer.device_addr.clone())
    };

    if let Some(device_addr) = device_addr {
        let message = serde_json::json!({
            "tag": FILE_TAG,
            "stat": "cancel",
        });
        send_interconnect_message(&device_addr, &message.to_string());
    }

    {
        let mut state = ui_state()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        set_status_message(&mut state, "已取消发送", StatusTone::Neutral, true);
        finish_transfer(&mut state, true);
    }
    render_from_state();
}

fn handle_handshake_message(message: &Value) {
    let count = message.get("count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let (device_addr, should_start, should_reply, timer_to_clear) = {
        let mut state = ui_state()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let (device_addr, should_start, should_reply) = {
            let Some(transfer) = state.transfer.as_mut() else {
                return;
            };
            if count > 0 {
                transfer.handshake_complete = true;
            }
            let should_start = transfer.pending_start && transfer.handshake_complete;
            if should_start {
                transfer.pending_start = false;
            }
            let should_reply = count < 2;
            (transfer.device_addr.clone(), should_start, should_reply)
        };
        let timer_to_clear = if count > 0 {
            state.handshake_timer_id.take()
        } else {
            None
        };
        (device_addr, should_start, should_reply, timer_to_clear)
    };

    if let Some(timer_id) = timer_to_clear {
        let _ = wit_bindgen::block_on(async { timer::clear_timer(timer_id).await });
    }

    if should_reply {
        send_handshake_message(&device_addr, count + 1);
    }

    if should_start {
        {
            let mut state = ui_state()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            set_status_message(
                &mut state,
                "已连接，开始传输...",
                StatusTone::Neutral,
                false,
            );
        }
        render_from_state();
        send_start_transfer();
    }
}

fn handle_file_message(message: &Value) {
    let payload = message.get("data").unwrap_or(message);
    let message_type = payload.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match message_type {
        "ready" => {
            let usage = payload.get("usage").and_then(|v| v.as_u64()).unwrap_or(0);
            if usage > 25 * 1024 * 1024 {
                {
                    let mut state = ui_state()
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    set_status_message(&mut state, "手环端存储空间不足", StatusTone::Error, true);
                    finish_transfer(&mut state, true);
                }
                render_from_state();
                return;
            }
            let found = payload
                .get("found")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let length = payload.get("length").and_then(|v| v.as_u64()).unwrap_or(0);
            if found && length > 0 {
                let current_chunk = (length as usize) / FILE_CHUNK_BYTES;
                send_next_chunk(current_chunk, true);
            } else {
                send_next_chunk(0, false);
            }
        }
        "error" => {
            let count = payload.get("count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            send_next_chunk(count, true);
        }
        "next" => {
            let count = payload.get("count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let total = {
                let state = ui_state().lock().unwrap_or_else(|p| p.into_inner());
                state.transfer.as_ref().map(|t| t.total_chunks).unwrap_or(0)
            };
            tracing::info!("recv next count={}, total_chunks={}", count, total);

            send_next_chunk(count, false);
        }
        "success" => {
            {
                let mut state = ui_state()
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.progress = 1.0;
                set_status_message(&mut state, "发送成功", StatusTone::Success, true);
                finish_transfer(&mut state, false);
            }
            render_from_state();
        }
        "cancel" => {
            {
                let mut state = ui_state()
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                set_status_message(&mut state, "传输已取消", StatusTone::Neutral, true);
                finish_transfer(&mut state, true);
            }
            render_from_state();
        }
        _ => {}
    }
}

fn send_start_transfer() {
    let (device_addr, message) = {
        let mut state = ui_state()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(transfer) = state.transfer.as_mut() else {
            return;
        };
        transfer.pending_start = false;
        let message = serde_json::json!({
            "tag": FILE_TAG,
            "stat": "startTransfer",
            "filename": transfer.file_name.clone(),
            "total": transfer.total_chunks,
            "chunkSize": FILE_CHUNK_BYTES,
        });
        (transfer.device_addr.clone(), message.to_string())
    };

    if !send_interconnect_message(&device_addr, &message) {
        {
            let mut state = ui_state()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            set_status_message(&mut state, "发送失败，请重试", StatusTone::Error, true);
            finish_transfer(&mut state, true);
        }
        render_from_state();
    }
}

fn send_next_chunk(current_chunk: usize, is_resend: bool) {
    let (device_addr, chunk, total_chunks, speed_text) = {
        let mut state = ui_state()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(transfer) = state.transfer.as_mut() else {
            return;
        };
        if current_chunk == transfer.total_chunks {
            let message = serde_json::json!({
                "tag": FILE_TAG,
                "stat": "d",
                "count": current_chunk,
                "data": "",
                "setCount": Value::Null,
            });
            send_interconnect_message(&transfer.device_addr, &message.to_string());
            return;
        }
        if current_chunk > transfer.total_chunks {
            return;
        }
        let chunk = match transfer.chunk(current_chunk) {
            Some(chunk) => chunk.to_string(),
            None => {
                set_status_message(&mut state, "分块失败", StatusTone::Error, true);
                finish_transfer(&mut state, true);
                drop(state);
                render_from_state();
                return;
            }
        };
        transfer.current_chunk = current_chunk;
        let speed_text = compute_speed_text(&mut transfer.last_chunk_time, chunk.as_bytes().len());
        (
            transfer.device_addr.clone(),
            chunk,
            transfer.total_chunks,
            speed_text,
        )
    };

    tracing::info!(
        "chunk {} len={} bytes",
        current_chunk,
        chunk.as_bytes().len()
    );

    let progress = current_chunk as f32 / total_chunks as f32;
    {
        let mut state = ui_state()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.progress = progress;
        state.speed_text = speed_text;
        let status = format!("发送中 {}%", (progress * 100.0).round());
        set_status_message(&mut state, &status, StatusTone::Neutral, false);
    }

    let message = serde_json::json!({
        "tag": FILE_TAG,
        "stat": "d",
        "count": current_chunk,
        "data": chunk,
        "setCount": if is_resend { Value::from(current_chunk as u64) } else { Value::Null },
    });

    if !send_interconnect_message(&device_addr, &message.to_string()) {
        {
            let mut state = ui_state()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            set_status_message(&mut state, "发送失败，请重试", StatusTone::Error, true);
            finish_transfer(&mut state, true);
        }
        render_from_state();
        return;
    }
    render_from_state();
}

fn send_handshake_message(device_addr: &str, count: usize) {
    let message = serde_json::json!({
        "tag": HANDSHAKE_TAG,
        "count": count,
    });
    send_interconnect_message(device_addr, &message.to_string());
}

fn send_interconnect_message(device_addr: &str, payload: &str) -> bool {
    wit_bindgen::block_on(async {
        interconnect::send_qaic_message(device_addr, PACKAGE_NAME, payload).await
    })
    .is_ok()
}

fn get_device_addr() -> Result<String, String> {
    let devices = wit_bindgen::block_on(async { device::get_connected_device_list().await });
    devices
        .first()
        .map(|device| device.addr.clone())
        .ok_or_else(|| "未找到设备".to_string())
}

fn get_app_info(device_addr: &str) -> Result<thirdpartyapp::AppInfo, String> {
    let app_list =
        wit_bindgen::block_on(async { thirdpartyapp::get_thirdparty_app_list(device_addr).await });
    let apps = app_list.map_err(|_| "获取应用列表失败".to_string())?;
    apps.into_iter()
        .find(|app| app.package_name == PACKAGE_NAME)
        .ok_or_else(|| "请先安装 BandBBS 客户端".to_string())
}

fn build_transfer_state(
    device_addr: String,
    file_name: String,
    text: String,
    bytes_len: usize,
) -> Result<TransferState, String> {
    if bytes_len == 0 {
        return Err("文件为空".to_string());
    }
    let total_chunks = (bytes_len + FILE_CHUNK_BYTES - 1) / FILE_CHUNK_BYTES;
    let total_chars = text.chars().count();
    if total_chars == 0 {
        return Err("文件为空".to_string());
    }
    let chunk_size_chars = (total_chars + total_chunks - 1) / total_chunks;
    let boundaries = build_boundaries(&text, chunk_size_chars, total_chunks);

    Ok(TransferState {
        device_addr,
        file_name,
        text,
        total_chunks,
        boundaries,
        current_chunk: 0,
        last_chunk_time: None,
        pending_start: true,
        handshake_complete: false,
    })
}

impl TransferState {
    fn chunk(&self, index: usize) -> Option<&str> {
        if index >= self.total_chunks {
            return None;
        }
        let start = *self.boundaries.get(index)?;
        let end = *self.boundaries.get(index + 1)?;
        self.text.get(start..end)
    }
}

fn build_boundaries(text: &str, chunk_size_chars: usize, total_chunks: usize) -> Vec<usize> {
    let mut boundaries = Vec::with_capacity(total_chunks + 1);
    boundaries.push(0);
    if chunk_size_chars == 0 {
        boundaries.push(text.len());
        return boundaries;
    }

    let mut count = 0usize;
    for (idx, _) in text.char_indices() {
        if count > 0 && count % chunk_size_chars == 0 && boundaries.len() < total_chunks {
            boundaries.push(idx);
        }
        count += 1;
    }

    boundaries.push(text.len());
    while boundaries.len() < total_chunks + 1 {
        boundaries.push(text.len());
    }
    boundaries
}

fn schedule_hide_message(state: &mut UiState) {
    clear_hide_message_timer(state);
    let timer_id =
        wit_bindgen::block_on(async { timer::set_timeout(3000, TIMER_HIDE_MESSAGE).await });
    state.hide_message_timer_id = Some(timer_id);
}

fn schedule_handshake_timeout(state: &mut UiState) {
    clear_handshake_timer(state);
    let timer_id =
        wit_bindgen::block_on(async { timer::set_timeout(3000, TIMER_HANDSHAKE_TIMEOUT).await });
    state.handshake_timer_id = Some(timer_id);
}

fn clear_hide_message_timer(state: &mut UiState) {
    if let Some(timer_id) = state.hide_message_timer_id.take() {
        let _ = wit_bindgen::block_on(async { timer::clear_timer(timer_id).await });
    }
}

fn clear_handshake_timer(state: &mut UiState) {
    if let Some(timer_id) = state.handshake_timer_id.take() {
        let _ = wit_bindgen::block_on(async { timer::clear_timer(timer_id).await });
    }
}

fn set_status_message(state: &mut UiState, message: &str, tone: StatusTone, auto_hide: bool) {
    state.status_message = Some(message.to_string());
    state.status_tone = tone;
    if auto_hide {
        schedule_hide_message(state);
    } else {
        clear_hide_message_timer(state);
    }
}

fn finish_transfer(state: &mut UiState, clear_progress: bool) {
    state.is_sending = false;
    state.transfer = None;
    if clear_progress {
        state.progress = 0.0;
    }
    state.speed_text = None;
    clear_handshake_timer(state);
}

fn clear_transfer_state(state: &mut UiState, clear_progress: bool) {
    state.transfer = None;
    state.is_sending = false;
    if clear_progress {
        state.progress = 0.0;
    }
    state.speed_text = None;
    clear_handshake_timer(state);
}

fn compute_speed_text(last_time: &mut Option<SystemTime>, chunk_bytes: usize) -> Option<String> {
    let now = SystemTime::now();
    let speed_text = last_time.and_then(|prev| {
        now.duration_since(prev).ok().and_then(|elapsed| {
            let secs = elapsed.as_secs_f64();
            if secs <= 0.0 {
                None
            } else {
                let speed = (chunk_bytes as f64 / secs) as usize;
                Some(format!("{}/s", format_bytes(speed)))
            }
        })
    });
    *last_time = Some(now);
    speed_text
}
fn extract_payload_text(payload: &str) -> String {
    if let Ok(json) = serde_json::from_str::<Value>(payload) {
        if let Some(text) = json.get("payloadText").and_then(|v| v.as_str()) {
            return text.to_string();
        }
        if let Some(payload_value) = json.get("payload") {
            if let Some(text) = payload_value.as_str() {
                return text.to_string();
            }
            return payload_value.to_string();
        }
    }
    payload.to_string()
}

fn format_bytes(bytes: usize) -> String {
    if bytes == 0 {
        return "0 Bytes".to_string();
    }
    let k = 1024_f64;
    let sizes = ["Bytes", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut idx = 0usize;
    while size >= k && idx < sizes.len() - 1 {
        size /= k;
        idx += 1;
    }
    format!("{:.2} {}", size, sizes[idx])
}

fn build_main_ui(state: &UiState) -> ui::Element {
    let file_label = ui::Element::new(ui::ElementType::P, Some("文件"))
        .size(12)
        .text_color("#666666")
        .margin_bottom(4);
    let file_info = match &state.file_name {
        Some(name) => format!("{} | {}", name, format_bytes(state.file_size_bytes)),
        None => "未选择文件".to_string(),
    };
    let file_info = ui::Element::new(ui::ElementType::P, Some(file_info.as_str()))
        .size(14)
        .margin_bottom(12);

    let mut pick_button = ui::Element::new(ui::ElementType::Button, Some("选择文件"))
        .bg("f0f0f0")
        .on(ui::Event::Click, EVENT_PICK_FILE);
    if state.is_sending {
        pick_button = pick_button.disabled();
    }

    let mut send_button = ui::Element::new(ui::ElementType::Button, Some("发送"))
        .bg("f0f0f0")
        .on(ui::Event::Click, EVENT_SEND_FILE);
    if state.is_sending || state.file_text.is_none() {
        send_button = send_button.disabled();
    }

    let mut cancel_button = ui::Element::new(ui::ElementType::Button, Some("取消"))
        .bg("f0f0f0")
        .on(ui::Event::Click, EVENT_CANCEL_SEND);
    if !state.is_sending {
        cancel_button = cancel_button.disabled();
    }

    let button_row = ui::Element::new(ui::ElementType::Div, None)
        .flex()
        .flex_direction(ui::FlexDirection::Row)
        .align_center()
        .child(pick_button)
        .child(send_button.margin_left(8))
        .child(cancel_button.margin_left(8))
        .margin_bottom(12);

    let speed_label = format!(
        "速率 {}",
        state.speed_text.clone().unwrap_or_else(|| "-".to_string())
    );
    let speed_text = ui::Element::new(ui::ElementType::P, Some(speed_label.as_str()))
        .size(12)
        .text_color("#666666");
    let progress_meta = ui::Element::new(ui::ElementType::Div, None)
        .flex()
        .flex_direction(ui::FlexDirection::Row)
        .align_center()
        .margin_bottom(6)
        .child(speed_text);

    let progress_width = if state.progress <= 0.0 {
        0
    } else {
        ((state.progress.clamp(0.0, 1.0) * 240.0).round() as u32).max(2)
    };
    let progress_fill = ui::Element::new(ui::ElementType::Div, None)
        .bg("#1781FF")
        .height(6)
        .width(progress_width)
        .radius(6)
        .transition("width 200ms ease");
    let progress_bar = ui::Element::new(ui::ElementType::Div, None)
        .bg("#F0F0F0")
        .radius(6)
        .width(240)
        .height(6)
        .child(progress_fill)
        .margin_bottom(12);

    let status_text = state
        .status_message
        .clone()
        .unwrap_or_else(|| " ".to_string());
    let mut status = ui::Element::new(ui::ElementType::P, Some(status_text.as_str()))
        .size(12)
        .margin_bottom(4);
    if state.status_message.is_some() {
        status = match state.status_tone {
            StatusTone::Success => status.text_color("#2E7D32"),
            StatusTone::Error => status.text_color("#B00020"),
            StatusTone::Neutral => status,
        };
    }

    ui::Element::new(ui::ElementType::Div, None)
        .flex()
        .flex_direction(ui::FlexDirection::Column)
        .padding(16)
        .border(1, "#1f1f1f")
        .radius(10)
        .width_full()
        .child(file_label)
        .child(file_info)
        .child(button_row)
        .child(progress_meta)
        .child(progress_bar)
        .child(status)
}

pub fn render_main_ui(element_id: &str) {
    let ui = {
        let mut state = ui_state()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.root_element_id = Some(element_id.to_string());
        build_main_ui(&state)
    };
    psys_host::ui::render(element_id, ui);
}

fn render_from_state() {
    let (root_id, ui) = {
        let state = ui_state()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        (state.root_element_id.clone(), build_main_ui(&state))
    };
    if let Some(root_id) = root_id {
        psys_host::ui::render(&root_id, ui);
    }
}
