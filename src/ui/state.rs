//! 插件 UI 状态：纯数据 + 纯日期换算，不含任何宿主调用（E-16 可在 host 上测试）。

use std::sync::{OnceLock, RwLock};

use chrono::Datelike;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventType {
    AddEvent,
    ModifyEvent,
    DeleteEvent,
    AddBackground,
}

impl EventType {
    pub const ALL: [EventType; 4] = [
        EventType::AddEvent,
        EventType::ModifyEvent,
        EventType::DeleteEvent,
        EventType::AddBackground,
    ];

    pub fn label(self) -> &'static str {
        match self {
            EventType::AddEvent => "添加事件",
            EventType::ModifyEvent => "修改事件",
            EventType::DeleteEvent => "删除事件",
            EventType::AddBackground => "添加背景",
        }
    }
}

/// 表单归属页：新增与修改共用一套字段定义，靠 scope 区分（E-18 去重）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormScope {
    Add,
    Modify,
    Delete,
}

impl FormScope {
    pub fn as_str(self) -> &'static str {
        match self {
            FormScope::Add => "add",
            FormScope::Modify => "modify",
            FormScope::Delete => "delete",
        }
    }

    pub fn tab(self) -> EventType {
        match self {
            FormScope::Add => EventType::AddEvent,
            FormScope::Modify | FormScope::Delete => EventType::ModifyEvent,
        }
    }

    /// 只有新增 / 修改有表单草稿，删除页只做事件选择
    pub fn has_form(self) -> bool {
        !matches!(self, FormScope::Delete)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventData {
    pub id: String,
    pub name: String,
    pub time: String,
    pub on_index: bool,
    pub if_staring_day: bool,
    pub theme_color: String,
}

impl EventData {
    pub fn draft(name: &str, time: &str, on_index: bool, if_staring_day: bool) -> Self {
        EventData {
            id: String::new(),
            name: name.to_string(),
            time: time.to_string(),
            on_index,
            if_staring_day,
            theme_color: String::new(),
        }
    }

    /// 下拉 / 列表展示用文案：名称 + 目标日，同名事件靠日期区分（E-19）
    pub fn label(&self) -> String {
        if self.name.is_empty() {
            return "(未命名)".to_string();
        }
        if self.time.is_empty() {
            return self.name.clone();
        }
        format!("{} · {}", self.name, self.time)
    }

    /// 选择键：优先稳定 ID，旧数据无 ID 时退回下标前缀（E-11）
    pub fn selection_key(&self, index: usize) -> String {
        if self.id.is_empty() {
            format!("idx:{}", index)
        } else {
            format!("id:{}", self.id)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Info,
    Success,
    Error,
}

/// 状态条：文案 + 三态类型，独立于表单草稿（E-19 固定位置与层级）
#[derive(Debug, Clone, PartialEq)]
pub struct StatusMessage {
    pub kind: StatusKind,
    pub text: String,
}

impl StatusMessage {
    pub fn info(text: &str) -> Self {
        StatusMessage { kind: StatusKind::Info, text: text.to_string() }
    }

    pub fn success(text: &str) -> Self {
        StatusMessage { kind: StatusKind::Success, text: text.to_string() }
    }

    pub fn error(text: &str) -> Self {
        StatusMessage { kind: StatusKind::Error, text: text.to_string() }
    }
}

/// 待发送消息：启动快应用后由定时器事件推进（E-04）
#[derive(Debug, Clone, PartialEq)]
pub struct PendingSend {
    pub token: u64,
    pub device_addr: String,
    pub payload: String,
    pub attempts: u32,
    pub in_progress: bool,
    pub purpose: SendPurpose,
    pub base_revision: Option<u64>,
    pub request_id: String,
    pub session_id: String,
    pub device_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendPurpose {
    Fetch,
    Mutation,
    Background,
}

/// 子页路由：概览 → 独立编辑子页 → 返回概览（对齐漫画「概览卡片 + 独立编辑子页」）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Overview,
    Editor(FormScope),
}

impl Route {
    pub fn is_editor(self) -> bool {
        matches!(self, Route::Editor(_))
    }
}

pub struct UiState {
    pub root_element_id: Option<String>,
    pub current_tab: EventType,
    pub route: Route,
    pub event_data: EventData,
    pub modify_event_data: EventData,
    pub all_events: Vec<EventData>,
    pub primary_id: Option<String>,
    pub revision: Option<u64>,
    pub selected_event_key: Option<String>,
    pub has_fetched_events: bool,
    pub event_page: usize,
    pub status: Option<StatusMessage>,
    pub pending_delete_key: Option<String>,
    pub message_timer_id: Option<u64>,
    pub today: String,
    /// 上次渲染的视图指纹：内容未变则不重建界面（E-17）
    pub rendered_fingerprint: Option<u64>,
    pub pending_send: Option<PendingSend>,
    pub send_sequence: u64,
    /// 命令已发出但还没从腕端列表回读核实；允许刷新，阻止重复增删改
    pub awaiting_confirmation: bool,
    pub awaiting_revision: Option<u64>,
    pub delete_dialog_open: bool,
    pub session_id: String,
    pub target_device: Option<String>,
    pub protocol_v2: bool,
    pub capabilities: Vec<String>,
    pub result_deadline: Option<u64>,
    /// 独立于 timer 的原请求身份；查询超时后仍接受迟到结果。
    pub result_request: Option<PendingSend>,
    pub snapshot_device: Option<String>,
    pub handshake_token: Option<u64>,
    pub handshake_ready: bool,
    pub handshake_polls: u32,
}

impl UiState {
    pub fn new(today: String) -> Self {
        UiState {
            root_element_id: None,
            current_tab: EventType::AddEvent,
            route: Route::Overview,
            event_data: EventData::draft("", &today, true, false),
            modify_event_data: EventData::default(),
            all_events: Vec::new(),
            primary_id: None,
            revision: None,
            selected_event_key: None,
            has_fetched_events: false,
            event_page: 0,
            status: None,
            pending_delete_key: None,
            message_timer_id: None,
            today,
            rendered_fingerprint: None,
            pending_send: None,
            send_sequence: 0,
            awaiting_confirmation: false,
            awaiting_revision: None,
            delete_dialog_open: false,
            session_id: String::new(),
            target_device: None,
            protocol_v2: false,
            capabilities: Vec::new(),
            result_deadline: None,
            result_request: None,
            snapshot_device: None,
            handshake_token: None,
            handshake_ready: false,
            handshake_polls: 0,
        }
    }

    /// 表单数据按 scope 取可变引用，新增 / 修改共用字段定义（E-18）
    pub fn draft_mut(&mut self, scope: FormScope) -> Option<&mut EventData> {
        match scope {
            FormScope::Add => Some(&mut self.event_data),
            FormScope::Modify => Some(&mut self.modify_event_data),
            FormScope::Delete => None,
        }
    }

    pub fn draft(&self, scope: FormScope) -> Option<&EventData> {
        match scope {
            FormScope::Add => Some(&self.event_data),
            FormScope::Modify => Some(&self.modify_event_data),
            FormScope::Delete => None,
        }
    }

    pub fn key_of(&self, index: usize) -> Option<String> {
        self.all_events.get(index).map(|event| event.selection_key(index))
    }

    pub fn index_of_key(&self, key: &str) -> Option<usize> {
        for (index, event) in self.all_events.iter().enumerate() {
            if event.selection_key(index) == key || event.id == key {
                return Some(index);
            }
        }
        None
    }

    /// 选中事件并载入修改表单
    pub fn select_event(&mut self, key: &str) -> bool {
        let Some(index) = self.index_of_key(key) else {
            return false;
        };
        let Some(event) = self.all_events.get(index).cloned() else {
            return false;
        };
        self.selected_event_key = Some(key.to_string());
        self.modify_event_data = event;
        true
    }

    /// 进入独立编辑子页：先把事件内容载入表单，再切路由
    pub fn open_editor(&mut self, key: &str) -> bool {
        if !self.select_event(key) {
            return false;
        }
        self.route = Route::Editor(FormScope::Modify);
        true
    }

    pub fn back_to_overview(&mut self) {
        self.route = Route::Overview;
    }

    /// 正在与腕端交互：锁定重复派发（P1-51）
    pub fn busy(&self) -> bool {
        self.pending_send.is_some() || self.awaiting_confirmation || self.delete_dialog_open
    }

    pub fn sending(&self) -> bool {
        self.pending_send.is_some()
    }

    /// 原子预约单飞发送。腕端确认待回读时，只允许拉取列表，不允许重复 mutation。
    pub fn reserve_send(&mut self, payload: String, purpose: SendPurpose) -> Option<u64> {
        if self.pending_send.is_some()
            || self.delete_dialog_open
            || (self.awaiting_confirmation && purpose != SendPurpose::Fetch)
        {
            return None;
        }
        self.send_sequence = self.send_sequence.wrapping_add(1);
        let token = self.send_sequence;
        self.pending_send = Some(PendingSend {
            token,
            device_addr: String::new(),
            payload,
            attempts: 0,
            in_progress: true,
            purpose,
            base_revision: self.revision,
            request_id: format!("req-{}", token),
            session_id: self.session_id.clone(),
            device_id: self.target_device.clone().unwrap_or_default(),
        });
        Some(token)
    }

    pub fn attach_send_device(&mut self, token: u64, device_addr: String) -> bool {
        let Some(pending) = self.pending_send.as_ref().filter(|pending| pending.token == token) else {
            return false;
        };
        if (!pending.device_addr.is_empty() || pending.purpose != SendPurpose::Fetch)
            && !pending.device_id.is_empty() && pending.device_id != device_addr { return false; }
        let payload: serde_json::Value = serde_json::from_str(&pending.payload).unwrap_or_default();
        let from_snapshot = matches!(payload.get("type").and_then(|v| v.as_str()), Some("changeEvent" | "deleteEvent"));
        if from_snapshot && self.snapshot_device.as_deref() != Some(device_addr.as_str()) { return false; }
        if self.target_device.as_deref().is_some_and(|target| target != device_addr) {
            self.session_id = format!("{}-{}", self.session_id, self.send_sequence);
            self.all_events.clear();
            self.clear_selection();
            self.has_fetched_events = false;
            self.snapshot_device = None;
            self.revision = None;
            self.primary_id = None;
            self.protocol_v2 = false;
            self.capabilities.clear();
        }
        self.target_device = Some(device_addr.clone());
        let pending = self.pending_send.as_mut().unwrap();
        pending.device_addr = device_addr;
        pending.device_id = self.target_device.clone().unwrap_or_default();
        pending.session_id = self.session_id.clone();
        pending.in_progress = false;
        true
    }

    pub fn start_handshake(&mut self, token: u64) {
        self.handshake_token = Some(token);
        self.handshake_ready = false;
        self.handshake_polls = 0;
        self.protocol_v2 = false;
        self.capabilities.clear();
    }

    pub fn accept_capabilities(&mut self, value: &serde_json::Value) -> bool {
        let Some(pending) = self.pending_send.as_ref() else { return false; };
        if self.handshake_token != Some(pending.token) || self.handshake_ready
            || value.get("sessionId").and_then(|v| v.as_str()) != Some(pending.session_id.as_str())
            || self.target_device.as_deref() != Some(pending.device_id.as_str()) { return false; }
        self.protocol_v2 = value.get("protocolVersion").and_then(|v| v.as_u64()).unwrap_or(0) >= 2;
        self.capabilities = value.get("capabilities").and_then(|v| v.as_array()).map(|items|
            items.iter().filter_map(|item| item.as_str().map(str::to_owned)).collect()).unwrap_or_default();
        self.handshake_ready = true;
        true
    }

    pub fn supports(&self, capability: &str) -> bool {
        self.protocol_v2 && self.handshake_ready && self.capabilities.iter().any(|item| item == capability)
    }

    pub fn poll_handshake(&mut self, token: u64, max_polls: u32) -> bool {
        if self.handshake_token != Some(token) { return false; }
        if !self.handshake_ready && self.handshake_polls < max_polls {
            self.handshake_polls += 1;
            let attempts = self.pending_send.as_ref().map(|item| item.attempts).unwrap_or(0);
            self.retry_send(token, attempts);
            true
        } else {
            if !self.handshake_ready {
                self.handshake_ready = true;
                self.protocol_v2 = false;
                self.capabilities.clear();
            }
            false
        }
    }

    /// 定时器 token 只能领取一次；重复/过期 timer 不会重复发送。
    pub fn claim_send(&mut self, token: u64) -> Option<PendingSend> {
        let pending = self.pending_send.as_mut()?;
        if pending.token != token || pending.in_progress || pending.device_addr.is_empty() {
            return None;
        }
        pending.in_progress = true;
        Some(pending.clone())
    }

    pub fn retry_send(&mut self, token: u64, attempts: u32) -> bool {
        let Some(pending) = self.pending_send.as_mut().filter(|pending| pending.token == token) else {
            return false;
        };
        pending.attempts = attempts;
        pending.in_progress = false;
        true
    }

    pub fn finish_send(&mut self, token: u64) -> Option<PendingSend> {
        if self.pending_send.as_ref().map(|pending| pending.token) != Some(token) {
            return None;
        }
        if self.handshake_token == Some(token) { self.handshake_token = None; }
        self.pending_send.take()
    }

    pub fn apply_confirmed_list(&mut self, primary_id: Option<String>, revision: Option<u64>) {
        self.snapshot_device = self.target_device.clone();
        if self.pending_send.as_ref().is_some_and(|pending| pending.purpose == SendPurpose::Fetch) {
            // The list can arrive before the send-transport callback. Consume
            // the fetch token here so its later timer completion is harmless.
            self.pending_send = None;
        }
        if self.awaiting_confirmation && self.result_request.is_none() {
            let fresh_enough = match (revision, self.awaiting_revision) {
                (Some(current), Some(base)) => current >= base,
                // Old app versions don't send revision; retain compatibility.
                (None, _) | (Some(_), None) => true,
            };
            if fresh_enough {
                self.awaiting_confirmation = false;
                self.awaiting_revision = None;
            }
        }
        self.primary_id = primary_id;
        self.revision = revision;
    }

    /// Mutation send completion may arrive after an interconnect list response.
    /// Don't re-lock if the state already advanced beyond the captured revision.
    pub fn require_confirmation(&mut self, base_revision: Option<u64>) {
        let already_confirmed = matches!((self.revision, base_revision), (Some(current), Some(base)) if current > base);
        self.awaiting_confirmation = !already_confirmed;
        self.awaiting_revision = if already_confirmed { None } else { base_revision };
    }

    /// 必须在发送宿主 await 前预约，回执可能早于传输回调。
    pub fn begin_result_wait(&mut self, pending: &PendingSend) {
        self.result_request = Some(pending.clone());
        self.result_deadline = Some(pending.token);
        self.awaiting_confirmation = true;
        self.awaiting_revision = pending.base_revision;
    }

    pub fn apply_request_message(&mut self, value: &serde_json::Value) -> bool {
        let Some(request) = self.result_request.as_ref() else { return false; };
        if value.get("requestId").and_then(|v| v.as_str()) != Some(request.request_id.as_str())
            || value.get("sessionId").and_then(|v| v.as_str()) != Some(request.session_id.as_str())
            || value.get("deviceId").and_then(|v| v.as_str()) != Some(request.device_id.as_str())
            || self.target_device.as_deref() != Some(request.device_id.as_str())
        { return false; }
        let kind = value.get("type").and_then(|v| v.as_str());
        if kind == Some("requestStatus") {
            self.status = Some(StatusMessage::info(match value.get("status").and_then(|v| v.as_str()) {
                Some("processing") => "腕端仍在保存，请等待原请求结果",
                _ => "原请求结果未知，请核实腕端数据；不要重复新增",
            }));
            return true;
        }
        if kind != Some("mutationResult") { return false; }
        let Some(ok) = value.get("ok").and_then(|v| v.as_bool()) else { return false; };
        let revision = value.get("revision").and_then(|v| v.as_u64());
        if ok && matches!((revision, request.base_revision), (Some(current), Some(base)) if current < base) {
            return false;
        }
        let token = request.token;
        let code = value.get("code").and_then(|v| v.as_str()).unwrap_or("");
        if !ok && code == "REQUEST_CONFLICT" {
            self.status = Some(StatusMessage::error("请求身份冲突，原请求可能仍在保存；请查询原结果，不要重复新增"));
            return true;
        }
        let partial = value.get("primary").and_then(|v| v.get("ok")).and_then(|v| v.as_bool()) == Some(false)
            || value.get("watchface").and_then(|v| v.get("ok")).and_then(|v| v.as_bool()) == Some(false)
            || code == "WATCHFACE_FAIL";
        self.finish_send(token);
        self.result_request = None;
        self.result_deadline = None;
        self.awaiting_confirmation = false;
        self.awaiting_revision = None;
        if ok {
            if let Some(revision) = revision { self.revision = Some(self.revision.unwrap_or(0).max(revision)); }
            self.status = Some(if partial {
                StatusMessage::error("事件已保存，但主事件或表盘同步失败；请核实分项结果，不要重复新增")
            } else { StatusMessage::success("腕端已保存") });
        } else {
            self.status = Some(StatusMessage::error(&format!("腕端拒绝保存（{}），可修正后重试", code)));
            if code == "REQUEST_CACHE_FULL" {
                // 明确拒绝且原请求未执行，下一次显式操作使用新会话。
                self.session_id = format!("{}-next-{}", self.session_id, token);
                self.protocol_v2 = false;
                self.capabilities.clear();
            }
        }
        true
    }

    pub fn clear_selection(&mut self) {
        self.selected_event_key = None;
        self.pending_delete_key = None;
        self.modify_event_data = EventData::default();
    }

    pub fn remove_selected(&mut self) -> bool {
        let Some(key) = self.selected_event_key.clone() else {
            return false;
        };
        let Some(index) = self.index_of_key(&key) else {
            self.clear_selection();
            return false;
        };
        self.all_events.remove(index);
        self.clear_selection();
        self.event_page = self.event_page.min(self.max_page().saturating_sub(1));
        true
    }

    pub fn max_page(&self) -> usize {
        if self.all_events.is_empty() {
            1
        } else {
            self.all_events.len().div_ceil(crate::ui::theme::EVENT_PAGE_SIZE)
        }
    }

    pub fn page_events(&self) -> Vec<(usize, &EventData)> {
        let size = crate::ui::theme::EVENT_PAGE_SIZE;
        let page = self.event_page.min(self.max_page().saturating_sub(1));
        let start = page * size;
        let end = (start + size).min(self.all_events.len());
        if start >= end {
            return Vec::new();
        }
        self.all_events[start..end].iter().enumerate().map(|(offset, event)| (start + offset, event)).collect()
    }
}

static UI_STATE: OnceLock<RwLock<UiState>> = OnceLock::new();

pub fn ui_state() -> &'static RwLock<UiState> {
    UI_STATE.get_or_init(|| RwLock::new(UiState::new(today_from_offset(0))))
}

/// 用新的「今天」重置全部状态（宿主加载时对齐时区，测试时隔离用例）
pub fn reset_state(today: &str) {
    let mut state = ui_state().write().unwrap_or_else(|poisoned| poisoned.into_inner());
    *state = UiState::new(today.to_string());
}

pub fn read_state() -> std::sync::RwLockReadGuard<'static, UiState> {
    ui_state().read().unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn write_state() -> std::sync::RwLockWriteGuard<'static, UiState> {
    ui_state().write().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 纯日期换算：UTC 时刻 + 时区偏移 → YYYY-MM-DD（宿主只负责取偏移）
pub fn local_date(utc_now: chrono::DateTime<chrono::Utc>, offset_minutes: i32) -> String {
    let offset = chrono::FixedOffset::east_opt(offset_minutes * 60)
        .unwrap_or_else(|| chrono::FixedOffset::east_opt(0).unwrap());
    let local = utc_now.with_timezone(&offset);
    format!("{:04}-{:02}-{:02}", local.year(), local.month(), local.day())
}

fn today_from_offset(offset_minutes: i32) -> String {
    local_date(chrono::Utc::now(), offset_minutes)
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use super::*;

    fn state_with_events() -> UiState {
        let mut state = UiState::new("2026-10-06".to_string());
        state.all_events = vec![
            EventData { id: "e1".into(), name: "同名".into(), time: "2026-01-01".into(), ..Default::default() },
            EventData { id: "e2".into(), name: "同名".into(), time: "2026-02-02".into(), ..Default::default() },
            EventData { name: "无ID".into(), time: "2026-03-03".into(), ..Default::default() },
        ];
        state
    }

    fn waiting_result() -> (UiState, u64) {
        let mut state = UiState::new("2026-10-07".into());
        state.session_id = "session".into();
        state.revision = Some(10);
        let token = state.reserve_send("{}".into(), SendPurpose::Mutation).unwrap();
        state.attach_send_device(token, "watch".into());
        let pending = state.claim_send(token).unwrap();
        state.begin_result_wait(&pending);
        (state, token)
    }

    #[test]
    fn 操作预约后不允许重绑到其他设备_旧列表不能用于新设备() {
        let mut state = state_with_events();
        state.target_device = Some("a".into());
        state.snapshot_device = Some("a".into());
        let token = state.reserve_send(r#"{"type":"deleteEvent","id":"e1"}"#.into(), SendPurpose::Mutation).unwrap();
        assert!(!state.attach_send_device(token, "b".into()));
        assert_eq!(state.target_device.as_deref(), Some("a"));
        assert_eq!(state.all_events.len(), 3);
        assert!(state.attach_send_device(token, "a".into()));
        assert!(!state.attach_send_device(token, "b".into()));
        state.finish_send(token);
        state.target_device = Some("b".into());
        let token = state.reserve_send(r#"{"type":"changeEvent","id":"e1"}"#.into(), SendPurpose::Mutation).unwrap();
        assert!(!state.attach_send_device(token, "b".into()), "快照属于 a，不能在 b 操作");
    }

    #[test]
    fn 首次握手等待后按逐项能力启用_旧会话与迟到回包不覆盖() {
        let (mut state, token) = waiting_result();
        state.start_handshake(token);
        assert!(!state.supports("requestId"));
        assert!(state.poll_handshake(token, 8));
        let mut capabilities = serde_json::json!({"sessionId":"stale", "protocolVersion":2,
            "capabilities":["requestId", "stableId"]});
        assert!(!state.accept_capabilities(&capabilities));
        capabilities["sessionId"] = serde_json::json!("session");
        assert!(state.accept_capabilities(&capabilities));
        assert!(!state.poll_handshake(token, 8));
        assert!(state.supports("requestId"));
        assert!(state.supports("stableId"));
        assert!(!state.supports("mutationResult"), "单项能力不能推断其他能力");
        capabilities["capabilities"] = serde_json::json!(["mutationResult"]);
        assert!(!state.accept_capabilities(&capabilities));
        assert!(!state.supports("mutationResult"));
    }

    #[test]
    fn 握手超时有界降级且迟到能力不改变本次发送模式() {
        let (mut state, token) = waiting_result();
        state.start_handshake(token);
        for _ in 0..8 { assert!(state.poll_handshake(token, 8)); }
        assert!(!state.poll_handshake(token, 8));
        assert_eq!(state.handshake_polls, 8);
        assert!(!state.protocol_v2);
        assert!(!state.accept_capabilities(&serde_json::json!({"sessionId":"session", "protocolVersion":2,
            "capabilities":["requestId", "mutationResult"]})));
        state.finish_send(token);
        assert!(!state.poll_handshake(token, 8));
    }

    #[test]
    fn 显式刷新可切设备并清空旧快照与能力_已绑定刷新不再重绑() {
        let mut state = state_with_events();
        state.target_device = Some("a".into());
        state.snapshot_device = Some("a".into());
        state.session_id = "session-a".into();
        state.revision = Some(9);
        state.protocol_v2 = true;
        state.capabilities.push("stableId".into());
        let token = state.reserve_send(r#"{"type":"getAllEvent"}"#.into(), SendPurpose::Fetch).unwrap();
        assert!(state.attach_send_device(token, "b".into()));
        assert!(state.all_events.is_empty());
        assert!(state.snapshot_device.is_none());
        assert!(state.revision.is_none());
        assert!(state.capabilities.is_empty());
        assert_ne!(state.session_id, "session-a");
        assert!(!state.attach_send_device(token, "c".into()));
    }

    fn result_message(token: u64, ok: bool) -> serde_json::Value {
        serde_json::json!({"type":"mutationResult", "requestId":format!("req-{token}"),
            "sessionId":"session", "deviceId":"watch", "ok":ok, "revision":11})
    }

    #[test]
    fn 保存回执先于传输回调且重复回执不重新结算() {
        let (mut state, token) = waiting_result();
        let result = result_message(token, true);
        assert!(state.apply_request_message(&result));
        assert!(state.finish_send(token).is_none());
        assert!(!state.busy());
        assert_eq!(state.revision, Some(11));
        assert!(!state.apply_request_message(&result));
        let newer = state.reserve_send("new".into(), SendPurpose::Mutation).unwrap();
        assert!(!state.apply_request_message(&result));
        assert_eq!(state.pending_send.as_ref().unwrap().token, newer);
    }

    #[test]
    fn 查询后迟到回执仍接收_刷新不能解锁原请求() {
        let (mut state, token) = waiting_result();
        state.finish_send(token);
        state.result_deadline = None;
        state.apply_confirmed_list(None, Some(11));
        assert!(state.awaiting_confirmation);
        assert!(state.reserve_send("duplicate".into(), SendPurpose::Mutation).is_none());
        let status = serde_json::json!({"type":"requestStatus", "requestId":format!("req-{token}"),
            "sessionId":"session", "deviceId":"watch", "status":"processing"});
        assert!(state.apply_request_message(&status));
        assert!(state.awaiting_confirmation);
        assert!(state.apply_request_message(&result_message(token, true)));
        assert!(!state.busy());
    }

    #[test]
    fn 明确失败解锁_表盘分项失败不提示重复新增() {
        let (mut state, token) = waiting_result();
        let mut failed = result_message(token, false);
        failed["code"] = serde_json::json!("NAME_TOO_LONG");
        assert!(state.apply_request_message(&failed));
        assert!(!state.busy());
        let (mut state, token) = waiting_result();
        let mut partial = result_message(token, true);
        partial["watchface"] = serde_json::json!({"ok":false});
        assert!(state.apply_request_message(&partial));
        assert!(!state.busy());
        assert_eq!(state.status.as_ref().unwrap().kind, StatusKind::Error);
        assert!(state.status.as_ref().unwrap().text.contains("事件已保存"));
    }

    #[test]
    fn 原请求未知保留锁_错误身份和旧revision不结算() {
        let (mut state, token) = waiting_result();
        let mut result = result_message(token, true);
        result["sessionId"] = serde_json::json!("stale");
        assert!(!state.apply_request_message(&result));
        result["sessionId"] = serde_json::json!("session");
        result["deviceId"] = serde_json::json!("other");
        assert!(!state.apply_request_message(&result));
        result["deviceId"] = serde_json::json!("watch");
        result["revision"] = serde_json::json!(9);
        assert!(!state.apply_request_message(&result));
        result["type"] = serde_json::json!("requestStatus");
        result["status"] = serde_json::json!("unknown");
        assert!(state.apply_request_message(&result));
        assert!(state.busy());
        result["type"] = serde_json::json!("mutationResult");
        result["ok"] = serde_json::json!(false);
        result["code"] = serde_json::json!("REQUEST_CONFLICT");
        assert!(state.apply_request_message(&result));
        assert!(state.busy(), "冲突拒绝不代表原请求未执行");
    }

    #[test]
    fn 缓存满额明确拒绝后下一次操作切新会话() {
        let (mut state, token) = waiting_result();
        let mut result = result_message(token, false);
        result["code"] = serde_json::json!("REQUEST_CACHE_FULL");
        assert!(state.apply_request_message(&result));
        assert!(!state.busy());
        assert_ne!(state.session_id, "session");
        assert!(!state.protocol_v2);
        assert!(state.reserve_send("next".into(), SendPurpose::Mutation).is_some());
    }

    #[test]
    fn 展示文案带日期以区分同名事件() {
        let state = state_with_events();
        assert_eq!(state.all_events[0].label(), "同名 · 2026-01-01");
        assert_eq!(state.all_events[2].label(), "无ID · 2026-03-03");
        assert_eq!(EventData::default().label(), "(未命名)");
    }

    #[test]
    fn 选择键优先稳定ID_旧数据退回下标() {
        let state = state_with_events();
        assert_eq!(state.key_of(0).unwrap(), "id:e1");
        assert_eq!(state.key_of(2).unwrap(), "idx:2");
        assert_eq!(state.index_of_key("id:e2"), Some(1));
        assert_eq!(state.index_of_key("idx:2"), Some(2));
        assert_eq!(state.index_of_key("id:不存在"), None);
    }

    #[test]
    fn 选择失效返回false_不静默改错事件() {
        let mut state = state_with_events();
        assert!(state.select_event("id:e1"));
        assert_eq!(state.modify_event_data.time, "2026-01-01");
        assert!(!state.select_event("id:已过期"));
        assert_eq!(state.selected_event_key.as_deref(), Some("id:e1"), "失败选择不得覆盖既有选择");
    }

    #[test]
    fn 删除按选择键并清理选中态() {
        let mut state = state_with_events();
        assert!(state.select_event("id:e1"));
        assert!(state.remove_selected());
        assert_eq!(state.all_events.len(), 2);
        assert!(state.selected_event_key.is_none());
        assert!(!state.remove_selected(), "重复删除不得再次生效");
    }

    #[test]
    fn 分页有界且页码越界自动收敛() {
        let mut state = UiState::new("2026-10-06".to_string());
        assert_eq!(state.max_page(), 1);
        for index in 0..20 {
            state.all_events.push(EventData {
                id: format!("e{}", index),
                name: format!("事件{}", index),
                time: "2026-01-01".into(),
                ..Default::default()
            });
        }
        assert_eq!(state.max_page(), 3);
        assert_eq!(state.page_events().len(), 8);
        state.event_page = 2;
        assert_eq!(state.page_events().len(), 4);
        state.event_page = 9;
        assert_eq!(state.page_events().len(), 4, "越界页码回退到最后一页");
    }

    #[test]
    fn 表单按scope读写() {
        let mut state = UiState::new("2026-10-06".to_string());
        state.draft_mut(FormScope::Add).unwrap().name = "新增".into();
        state.draft_mut(FormScope::Modify).unwrap().name = "修改".into();
        assert_eq!(state.draft(FormScope::Add).unwrap().name, "新增");
        assert_eq!(state.draft(FormScope::Modify).unwrap().name, "修改");
    }

    #[test]
    fn 本地日期按偏移换算() {
        use chrono::TimeZone;
        let utc = chrono::Utc.with_ymd_and_hms(2026, 10, 6, 16, 30, 0).unwrap();
        assert_eq!(local_date(utc, 0), "2026-10-06");
        assert_eq!(local_date(utc, 480), "2026-10-07", "东八区跨日");
        assert_eq!(local_date(utc, -300), "2026-10-06");
        assert_eq!(local_date(utc, 900), "2026-10-07", "非法偏移回退 UTC");
    }

    #[test]
    fn 删除页没有表单草稿() {
        let state = UiState::new("2026-10-06".to_string());
        assert!(state.draft(FormScope::Add).is_some());
        assert!(state.draft(FormScope::Modify).is_some());
        assert!(state.draft(FormScope::Delete).is_none());
        assert!(!FormScope::Delete.has_form());
    }

    #[test]
    fn 编辑子页与概览之间可往返() {
        let mut state = state_with_events();
        assert_eq!(state.route, Route::Overview);
        assert!(state.open_editor("id:e1"));
        assert_eq!(state.route, Route::Editor(FormScope::Modify));
        assert!(state.route.is_editor());
        assert_eq!(state.modify_event_data.time, "2026-01-01");
        assert!(!state.open_editor("id:不存在"), "失效键不得进入子页");
        assert_eq!(state.route, Route::Editor(FormScope::Modify), "失败进入不改路由");
        state.back_to_overview();
        assert_eq!(state.route, Route::Overview);
    }

    #[test]
    fn 在途时判定为忙() {
        let mut state = UiState::new("2026-10-06".to_string());
        assert!(!state.busy());
        state.reserve_send("{}".into(), SendPurpose::Fetch).unwrap();
        assert!(state.busy());
    }

    #[test]
    fn 单飞预约在任何异步步骤之前锁住重复操作() {
        let mut state = UiState::new("2026-10-06".into());
        let token = state.reserve_send("add".into(), SendPurpose::Mutation).unwrap();
        assert_eq!(token, 1);
        assert!(state.busy());
        assert!(state.reserve_send("duplicate".into(), SendPurpose::Mutation).is_none());
        assert!(state.claim_send(token).is_none(), "设备信息未绑定时 timer 不得发送");
        assert!(state.attach_send_device(token, "watch".into()));
        let claimed = state.claim_send(token).unwrap();
        assert_eq!(claimed.payload, "add");
        assert!(state.claim_send(token).is_none(), "同一 timer token 只能领取一次");
    }

    #[test]
    fn 请求身份固定且换设备必须清空旧快照() {
        let mut state = UiState::new("2026-10-06".into());
        state.session_id = "session-a".into();
        let token = state.reserve_send("{}".into(), SendPurpose::Mutation).unwrap();
        let pending = state.pending_send.as_ref().unwrap();
        assert_eq!(pending.request_id, format!("req-{token}"));
        assert_eq!(pending.session_id, "session-a");
        state.target_device = Some("device-a".into());
        state.all_events.push(EventData {id:"e1".into(), name:"old".into(), ..Default::default()});
        state.target_device = Some("device-b".into());
        state.all_events.clear();
        state.clear_selection();
        state.has_fetched_events = false;
        assert!(state.all_events.is_empty());
        assert!(!state.has_fetched_events);
    }

    #[test]
    fn mutation等待回读期间只允许fetch() {
        let mut state = UiState::new("2026-10-06".into());
        let token = state.reserve_send("add".into(), SendPurpose::Mutation).unwrap();
        state.attach_send_device(token, "watch".into());
        assert_eq!(state.finish_send(token).unwrap().purpose, SendPurpose::Mutation);
        state.awaiting_confirmation = true;
        assert!(state.busy());
        assert!(state.reserve_send("second add".into(), SendPurpose::Mutation).is_none());
        let fetch = state.reserve_send("getAllEvent".into(), SendPurpose::Fetch).unwrap();
        assert!(state.awaiting_confirmation, "发起 fetch 不能提前解除确认锁");
        state.finish_send(fetch).unwrap();
        state.apply_confirmed_list(Some("e1".into()), Some(2));
        assert!(!state.busy());
        assert_eq!(state.primary_id.as_deref(), Some("e1"));
        assert_eq!(state.revision, Some(2));
        state.awaiting_confirmation = true;
        state.apply_confirmed_list(None, None);
        assert_eq!(state.primary_id, None, "旧协议响应须清除陈旧元数据");
        assert_eq!(state.revision, None);
    }

    #[test]
    fn fetch回包可先于发送回调并消费token_旧列表不释放mutation() {
        let mut state = UiState::new("2026-10-06".into());
        let fetch = state.reserve_send("getAllEvent".into(), SendPurpose::Fetch).unwrap();
        state.attach_send_device(fetch, "watch".into());
        state.apply_confirmed_list(Some("e1".into()), Some(8));
        assert!(state.pending_send.is_none(), "fetch响应先到时应消费该token");
        assert!(!state.busy());

        let mutation = state.reserve_send("add".into(), SendPurpose::Mutation).unwrap();
        state.attach_send_device(mutation, "watch".into());
        state.finish_send(mutation).unwrap();
        state.awaiting_confirmation = true;
        state.awaiting_revision = Some(8);
        let fetch = state.reserve_send("getAllEvent".into(), SendPurpose::Fetch).unwrap();
        state.attach_send_device(fetch, "watch".into());
        state.apply_confirmed_list(Some("old".into()), Some(7));
        assert!(state.pending_send.is_none(), "对应fetch响应应消费fetch token");
        assert!(state.awaiting_confirmation, "列表回包不能释放未确认 mutation");
    }

    #[test]
    fn 旧revision回包不能解除mutation确认锁_相同版本的freshfetch允许重试() {
        let mut state = UiState::new("2026-10-06".into());
        state.revision = Some(8);
        let token = state.reserve_send("change".into(), SendPurpose::Mutation).unwrap();
        state.finish_send(token).unwrap();
        state.awaiting_confirmation = true;
        state.awaiting_revision = Some(8);
        state.apply_confirmed_list(Some("e1".into()), Some(8));
        assert!(!state.awaiting_confirmation, "相同 revision 的 fresh fetch 表示 mutation 未改变数据，可显式重试");
        state.awaiting_confirmation = true;
        state.awaiting_revision = Some(8);
        state.apply_confirmed_list(Some("e1".into()), Some(7));
        assert!(state.awaiting_confirmation, "低于基线的 revision 是旧列表");
        state.apply_confirmed_list(Some("e1".into()), Some(9));
        assert!(!state.awaiting_confirmation, "更高 revision 确认 mutation 已落盘");
    }

    #[test]
    fn 回包先于发送回调时不重新锁住已确认mutation() {
        let mut state = UiState::new("2026-10-06".into());
        state.revision = Some(10);
        state.apply_confirmed_list(Some("e1".into()), Some(11));
        state.require_confirmation(Some(10));
        assert!(!state.awaiting_confirmation, "回包已先确认更高版本");
    }

    #[test]
    fn 同一发送token不能领取两次也不能领取过期token() {
        let mut state = UiState::new("2026-10-06".into());
        let token = state.reserve_send("getAllEvent".into(), SendPurpose::Fetch).unwrap();
        assert!(state.attach_send_device(token, "watch".into()));
        assert!(state.claim_send(token).is_some());
        assert!(state.claim_send(token).is_none());
        assert!(state.finish_send(token + 1).is_none());
        assert!(state.finish_send(token).is_some());
    }

    #[test]
    fn 对话框打开时busy防止重复删除动作() {
        let mut state = UiState::new("2026-10-06".into());
        state.delete_dialog_open = true;
        assert!(state.busy());
        assert!(state.reserve_send("delete".into(), SendPurpose::Mutation).is_none());
        state.delete_dialog_open = false;
        assert!(!state.busy());
    }

    #[test]
    fn 选择其他页事件不重置分页() {
        let mut state = UiState::new("2026-10-06".into());
        for index in 0..10 {
            state.all_events.push(EventData {
                id: format!("e{}", index),
                name: format!("事件{}", index),
                time: "2026-01-01".into(),
                ..Default::default()
            });
        }
        state.event_page = 1;
        assert!(state.open_editor("id:e9"));
        assert_eq!(state.event_page, 1);
    }
}
