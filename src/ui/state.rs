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
    pub source_index: Option<usize>,
    /// 保留接收模型的未知字段；编辑命令不回传它们，避免覆盖腕端更新。
    pub extensions: std::collections::BTreeMap<String, serde_json::Value>,
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
            source_index: None,
            extensions: Default::default(),
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
            format!("idx:{}", self.source_index.unwrap_or(index))
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

#[derive(Debug, Clone)]
pub struct ListRequest {
    pub pending: PendingSend,
    pub identified: bool,
    pub batched: bool,
}

pub struct ListAssembly {
    pub metadata: serde_json::Value,
    pub batches: Vec<Option<serde_json::Value>>,
    pub bytes: usize,
    pub items: usize,
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
    pub background_bytes: Vec<u8>,
    pub background_offset: usize,
    pub background_frame: Option<String>,
    pub background_retries: u32,
    pub background_timer: u64,
    pub background_width: String,
    pub background_height: String,
    pub background_cover: bool,
    pub background_preview: Option<String>,
    pub background_detail: String,
    pub background_device: Option<String>,
    pub backup_preview: Option<super::backup::Preview>,
    pub snapshot_complete: bool,
    pub local_file_busy: bool,
    pub list_sort: String,
    pub list_category: String,
    pub list_archived: bool,
    pub list_period: String,
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
    pub list_request: Option<ListRequest>,
    pub snapshot_token: Option<u64>,
    pub list_assembly: Option<ListAssembly>,
    pub result_unknown: bool,
    pub recovery_snapshot: Option<u64>,
    pub query_sequence: u64,
    pub query_deadline: Option<u64>,
    pub result_status: Option<StatusMessage>,
    pub legacy_request: Option<PendingSend>,
}

impl UiState {
    pub fn new(today: String) -> Self {
        UiState {
            background_bytes: Vec::new(), background_offset: 0, background_frame: None,
            background_retries: 0, background_timer: 0,
            background_width: "192".into(), background_height: "490".into(),
            background_cover: true, background_preview: None, background_detail: String::new(),
            background_device: None,
            backup_preview: None, snapshot_complete: false, local_file_busy: false,
            list_sort: "created".into(), list_category: "all".into(), list_archived: false,
            list_period: "all".into(),
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
            list_request: None,
            snapshot_token: None,
            list_assembly: None,
            result_unknown: false,
            recovery_snapshot: None,
            query_sequence: 0,
            query_deadline: None,
            result_status: None,
            legacy_request: None,
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
        if self.modify_event_data.extensions.get("calendar").and_then(|v|v.as_str()) == Some("lunar") {
            let date = &self.modify_event_data.extensions["lunarDate"];
            let input = format!("{:04}-{:02}-{:02}",date["year"].as_i64().unwrap(),date["month"].as_u64().unwrap(),date["day"].as_u64().unwrap());
            self.modify_event_data.extensions.insert("lunarInput".into(),serde_json::json!(input));
        }
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
        self.pending_send.is_some() || self.awaiting_confirmation || self.delete_dialog_open || self.local_file_busy
    }

    pub fn sending(&self) -> bool {
        self.pending_send.is_some()
    }

    /// 原子预约单飞发送。腕端确认待回读时，只允许拉取列表，不允许重复 mutation。
    pub fn reserve_send(&mut self, payload: String, purpose: SendPurpose) -> Option<u64> {
        if self.pending_send.is_some()
            || self.local_file_busy
            || self.delete_dialog_open
            || (self.awaiting_confirmation && purpose != SendPurpose::Fetch)
        {
            return None;
        }
        self.send_sequence = self.send_sequence.wrapping_add(1);
        if purpose != SendPurpose::Fetch { self.result_status = None; }
        if purpose == SendPurpose::Fetch { self.list_request = None; self.list_assembly = None; }
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
        if self.result_request.as_ref().is_some_and(|original| original.device_id != device_addr) {
            return false;
        }
        if self.legacy_request.as_ref().is_some_and(|original| original.device_id != device_addr) { return false; }
        if (!pending.device_addr.is_empty() || pending.purpose != SendPurpose::Fetch)
            && !pending.device_id.is_empty() && pending.device_id != device_addr { return false; }
        let payload: serde_json::Value = serde_json::from_str(&pending.payload).unwrap_or_default();
        let from_snapshot = matches!(payload.get("type").and_then(|v| v.as_str()), Some("changeEvent" | "deleteEvent" | "moveEvent" | "setPrimary" | "restoreBackup"));
        if from_snapshot && self.snapshot_device.as_deref() != Some(device_addr.as_str()) { return false; }
        if from_snapshot && payload.get("index").is_some() && self.snapshot_token.is_none() { return false; }
        if pending.purpose == SendPurpose::Background && self.background_device.as_deref() != Some(device_addr.as_str()) { return false; }
        if self.target_device.as_deref().is_some_and(|target| target != device_addr) {
            self.session_id = format!("{}-{}", self.session_id, self.send_sequence);
            self.all_events.clear();
            self.clear_selection();
            self.has_fetched_events = false;
            self.snapshot_device = None;
            self.snapshot_token = None;
            self.list_request = None;
            self.revision = None;
            self.primary_id = None;
            self.snapshot_complete = false;
            self.backup_preview = None;
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

    pub fn active_send(&self, token: u64) -> bool {
        self.pending_send.as_ref().is_some_and(|pending| pending.token == token)
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
        if value.get("handshakeId").and_then(|v| v.as_str()) != Some(format!("hello-{}", pending.token).as_str())
            || value.get("deviceId").and_then(|v| v.as_str()) != Some(pending.device_id.as_str()) {
            // 无标记的旧能力回包不提前结束等待；有界超时后走旧协议。
            return false;
        }
        self.protocol_v2 = value.get("protocolVersion").and_then(|v| v.as_u64()).unwrap_or(0) >= 2;
        self.capabilities = value.get("capabilities").and_then(|v| v.as_array()).map(|items|
            items.iter().filter_map(|item| item.as_str().map(str::to_owned)).collect()).unwrap_or_default();
        self.handshake_ready = true;
        let dimensions = value.get("screenWidth").and_then(|v|v.as_u64()).zip(value.get("screenHeight").and_then(|v|v.as_u64()));
        if let Some((width, height)) = dimensions.filter(|(w,h)| (1..=2048).contains(w) && (1..=2048).contains(h)) {
            let width = width.to_string();
            let height = height.to_string();
            if self.background_device.as_deref() != Some(pending.device_id.as_str()) || self.background_width != width || self.background_height != height {
                self.background_preview = None;
                self.background_detail.clear();
            }
            self.background_width = width;
            self.background_height = height;
            self.background_device = Some(pending.device_id.clone());
        } else {
            self.background_device = None;
            self.background_preview = None;
        }
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
        if self.awaiting_confirmation && self.result_request.is_none() && self.legacy_request.is_none() {
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
        self.legacy_request = None;
        self.query_deadline = None;
        self.result_status = None;
        self.result_unknown = false;
        self.recovery_snapshot = None;
        self.result_request = Some(pending.clone());
        self.result_deadline = Some(pending.token);
        self.awaiting_confirmation = true;
        self.awaiting_revision = pending.base_revision;
    }

    pub fn begin_legacy_wait(&mut self, pending: &PendingSend) {
        self.legacy_request = Some(pending.clone());
        self.result_unknown = true;
        self.recovery_snapshot = None;
        self.awaiting_confirmation = true;
        self.awaiting_revision = pending.base_revision;
        self.result_status = Some(StatusMessage::info("旧协议无法确认保存，请同步完整列表并核实；不要直接重复新增"));
    }

    pub fn expire_preparation(&mut self, token: u64) -> bool {
        if self.result_request.as_ref().is_some_and(|request| request.token == token)
            || self.legacy_request.as_ref().is_some_and(|request| request.token == token)
            || self.list_request.as_ref().is_some_and(|request| request.pending.token == token) { return false; }
        if self.finish_send(token).is_none() { return false; }
        self.status = Some(StatusMessage::error("启动或能力协商超时，业务消息未发送，可重新操作"));
        true
    }

    pub fn begin_list_wait(&mut self, pending: &PendingSend, identified: bool) {
        if self.list_request.as_ref().is_none_or(|request| request.pending.token != pending.token) {
            self.list_assembly = None;
        }
        self.list_request = Some(ListRequest { pending: pending.clone(), identified, batched: false });
    }

    pub fn expire_list(&mut self, token: u64) -> bool {
        if !self.list_request.as_ref().is_some_and(|request| request.pending.token == token) { return false; }
        self.list_request = None;
        self.list_assembly = None;
        self.finish_send(token);
        self.status = Some(StatusMessage::error("腕端列表接收超时，请重新同步；保留上次数据和草稿"));
        true
    }

    /// 所有验证先完成，再切换快照；刷新不把远端数据写进编辑草稿。
    pub fn accept_event_list(&mut self, parsed: crate::ui::protocol::ParsedEvents) -> bool {
        let Some(request) = self.list_request.as_ref() else { return false; };
        let pending = &request.pending;
        if self.target_device.as_deref() != Some(pending.device_id.as_str()) || self.session_id != pending.session_id {
            return false;
        }
        if request.identified {
            if parsed.revision.is_none() || parsed.request_id.as_deref() != Some(pending.request_id.as_str())
                || parsed.session_id.as_deref() != Some(pending.session_id.as_str())
                || parsed.device_id.as_deref() != Some(pending.device_id.as_str()) { return false; }
        } else if parsed.request_id.as_deref().is_some_and(|id| id != pending.request_id)
            || parsed.session_id.as_deref().is_some_and(|id| id != pending.session_id)
            || parsed.device_id.as_deref().is_some_and(|id| id != pending.device_id) { return false; }
        let minimum = self.revision.into_iter().chain(pending.base_revision).max();
        if matches!((parsed.revision, minimum), (Some(current), Some(base)) if current < base) { return false; }
        let skipped = parsed.skipped;
        self.snapshot_complete = skipped == 0 && parsed.revision.is_some() && parsed.primary_id.is_some() && parsed.sort_mode.is_some()
            && parsed.events.iter().all(|e| !e.id.is_empty());
        self.snapshot_token = Some(pending.token);
        self.recovery_snapshot = None;
        if skipped == 0 && self.result_unknown && self.result_request.as_ref().or(self.legacy_request.as_ref()).is_some_and(|original|
            original.device_id == pending.device_id && pending.token > original.token) {
            self.recovery_snapshot = Some(pending.token);
        }
        self.list_request = None;
        self.list_assembly = None;
        let selected_before = self.selected_event_key.as_deref().and_then(|key|
            self.index_of_key(key).and_then(|index| self.all_events.get(index))).cloned();
        self.all_events = parsed.events;
        if let Some(mode) = parsed.sort_mode { self.list_sort = mode; }
        self.has_fetched_events = true;
        self.apply_confirmed_list(parsed.primary_id, parsed.revision);
        self.event_page = self.event_page.min(self.max_page().saturating_sub(1));
        if self.selected_event_key.as_deref().is_some_and(|key| self.index_of_key(key).is_none()) {
            // 选择失效，但保留用户未提交的表单，不自动装载其他事件。
            self.selected_event_key = None;
            self.pending_delete_key = None;
        }
        if let Some(previous) = selected_before.filter(|event| event.id.is_empty()) {
            let same = self.selected_event_key.as_deref().and_then(|key| self.index_of_key(key))
                .and_then(|index| self.all_events.get(index)).is_some_and(|current|
                    current.name == previous.name && current.time == previous.time
                    && current.on_index == previous.on_index && current.if_staring_day == previous.if_staring_day
                    && current.theme_color == previous.theme_color);
            if !same { self.selected_event_key = None; self.pending_delete_key = None; }
        }
        self.status = Some(if self.result_request.is_some() {
            StatusMessage::info("列表已刷新，原请求结果仍待核实；不要重复新增")
        } else { StatusMessage::success(&format!("腕端列表已同步（跳过 {} 条无效数据）", skipped)) });
        true
    }

    pub fn expire_snapshot(&mut self, token: u64) {
        if self.snapshot_token == Some(token) { self.snapshot_token = None; }
        if self.recovery_snapshot == Some(token) { self.recovery_snapshot = None; }
    }

    pub fn can_finish_unknown(&self) -> bool {
        self.result_unknown && !self.sending() && self.list_request.is_none()
            && self.recovery_snapshot.is_some() && self.recovery_snapshot == self.snapshot_token
            && self.result_request.as_ref().or(self.legacy_request.as_ref()).is_some_and(|original|
                self.target_device.as_deref() == Some(original.device_id.as_str())
                && self.snapshot_device.as_deref() == Some(original.device_id.as_str()))
    }

    pub fn finish_unknown_after_review(&mut self) -> bool {
        if !self.can_finish_unknown() { return false; }
        self.result_request = None;
        self.legacy_request = None;
        self.result_deadline = None;
        self.result_unknown = false;
        self.recovery_snapshot = None;
        self.query_deadline = None;
        self.result_status = None;
        self.awaiting_confirmation = false;
        self.awaiting_revision = None;
        self.status = Some(StatusMessage::info("已结束原请求等待，未重发操作；草稿已保留，请按核实结果决定下一步"));
        true
    }

    pub fn begin_result_query(&mut self) -> Option<(u64, PendingSend)> {
        let original = self.result_request.as_ref()?;
        if self.target_device.as_deref() != Some(original.device_id.as_str()) { return None; }
        let original = original.clone();
        self.query_sequence = self.query_sequence.wrapping_add(1);
        self.query_deadline = Some(self.query_sequence);
        Some((self.query_sequence, original))
    }

    pub fn expire_result_query(&mut self, sequence: u64) -> bool {
        if self.query_deadline != Some(sequence) || self.result_request.is_none() { return false; }
        self.query_deadline = None;
        self.result_unknown = true;
        self.recovery_snapshot = None;
        self.result_status = Some(StatusMessage::info("原结果查询无响应；请重新同步同设备完整列表并核实，不能直接重复新增"));
        true
    }

    pub fn accept_list_batch(&mut self, value: &serde_json::Value) -> bool {
        let Some(request) = self.list_request.as_ref() else { return false; };
        if !request.identified
            || value.get("requestId").and_then(|v| v.as_str()) != Some(request.pending.request_id.as_str())
            || value.get("sessionId").and_then(|v| v.as_str()) != Some(request.pending.session_id.as_str())
            || value.get("deviceId").and_then(|v| v.as_str()) != Some(request.pending.device_id.as_str()) { return false; }
        let token = request.pending.token;
        if value.get("type").and_then(|v| v.as_str()) == Some("eventListError") {
            self.expire_list(token);
            self.status = Some(StatusMessage::error("腕端列表传输失败，保留上次列表和草稿"));
            return true;
        }
        let count = value.get("batchCount").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let index = value.get("batchIndex").and_then(|v| v.as_u64()).unwrap_or(u64::MAX) as usize;
        let total = value.get("total").and_then(|v| v.as_u64()).unwrap_or(u64::MAX) as usize;
        let data = value.get("data").and_then(|v| v.as_array());
        let size = value.to_string().len();
        let fail = |state: &mut Self| {
            state.expire_list(token);
            state.status = Some(StatusMessage::error("分批列表校验失败，保留上次列表和草稿"));
            true
        };
        if count == 0 || count > 200 || index >= count || total > 200 || size > 8192
            || data.is_none_or(|items| items.len() > 16)
            || value.get("revision").and_then(|v| v.as_u64()).is_none() { return fail(self); }
        let mut metadata = value.clone();
        metadata.as_object_mut().unwrap().remove("data");
        metadata.as_object_mut().unwrap().remove("batchIndex");
        if self.list_assembly.is_none() {
            self.list_assembly = Some(ListAssembly { metadata: metadata.clone(), batches: vec![None; count], bytes: 0, items: 0 });
        }
        let assembly = self.list_assembly.as_mut().unwrap();
        if assembly.metadata != metadata { return fail(self); }
        if let Some(prior) = &assembly.batches[index] {
            if prior != &value["data"] { return fail(self); }
            return false;
        }
        assembly.bytes += size;
        assembly.items += data.unwrap().len();
        if assembly.bytes > crate::ui::protocol::MAX_MESSAGE_BYTES || assembly.items > total || assembly.items > 200 { return fail(self); }
        assembly.batches[index] = Some(value["data"].clone());
        if assembly.batches.iter().any(Option::is_none) { return false; }
        let mut items = Vec::new();
        for batch in &assembly.batches { items.extend(batch.as_ref().unwrap().as_array().unwrap().iter().cloned()); }
        if items.len() != total { return fail(self); }
        let mut complete = assembly.metadata.clone();
        complete["data"] = serde_json::Value::Array(items);
        match crate::ui::protocol::parse_event_list(&complete.to_string()) {
            Ok(parsed) => {
                if self.accept_event_list(parsed) { true } else { fail(self) }
            }
            Err(_) => fail(self),
        }
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
            if !matches!(value.get("status").and_then(|v| v.as_str()), Some("processing" | "unknown" | "expired")) { return false; }
            self.query_deadline = None;
            let unknown = matches!(value.get("status").and_then(|v| v.as_str()), Some("unknown" | "expired"));
            if !unknown || !self.result_unknown { self.recovery_snapshot = None; }
            self.result_unknown = unknown;
            self.status = Some(StatusMessage::info(match value.get("status").and_then(|v| v.as_str()) {
                Some("processing") => "腕端仍在保存，请等待原请求结果",
                _ => "原请求结果未知，请核实腕端数据；不要重复新增",
            }));
            self.result_status = self.status.clone();
            return true;
        }
        if kind != Some("mutationResult") { return false; }
        let Some(ok) = value.get("ok").and_then(|v| v.as_bool()) else { return false; };
        let revision = value.get("revision").and_then(|v| v.as_u64());
        if ok && matches!((revision, request.base_revision), (Some(current), Some(base)) if current < base) {
            return false;
        }
        let token = request.token;
        let background_operation = request.purpose == SendPurpose::Background;
        let primary_operation = serde_json::from_str::<serde_json::Value>(&request.payload).ok()
            .is_some_and(|payload| payload["type"] == "setPrimary");
        let restore_operation = serde_json::from_str::<serde_json::Value>(&request.payload).ok()
            .is_some_and(|payload| payload["type"] == "restoreBackup");
        if primary_operation && ok && (revision.is_none()
            || value.get("primaryId").and_then(|v| v.as_str()).is_none()
            || value.get("primary").and_then(|v| v.get("ok")).and_then(|v| v.as_bool()) != Some(true)
            || value.get("watchface").and_then(|v| v.get("ok")).and_then(|v| v.as_bool()).is_none()) {
            return false;
        }
        let code = value.get("code").and_then(|v| v.as_str()).unwrap_or("");
        if !ok && code == "REQUEST_CONFLICT" {
            self.status = Some(StatusMessage::error("请求身份冲突，原请求可能仍在保存；请查询原结果，不要重复新增"));
            self.result_status = self.status.clone();
            return true;
        }
        let partial = value.get("primary").and_then(|v| v.get("ok")).and_then(|v| v.as_bool()) == Some(false)
            || value.get("watchface").and_then(|v| v.get("ok")).and_then(|v| v.as_bool()) == Some(false)
            || code == "WATCHFACE_FAIL";
        self.finish_send(token);
        self.result_request = None;
        self.query_deadline = None;
        self.result_unknown = false;
        self.recovery_snapshot = None;
        self.result_deadline = None;
        self.awaiting_confirmation = false;
        if background_operation {
            self.background_bytes.clear();
            self.background_frame = None;
            self.background_timer += 1;
        }
        self.awaiting_revision = None;
        if ok {
            self.snapshot_complete = false;
            if restore_operation { self.snapshot_complete = false; self.backup_preview = None; }
            if let Some(primary_id) = value.get("primaryId").and_then(|v| v.as_str()) {
                if revision.is_some_and(|r| self.revision.is_none_or(|current| r >= current)) {
                    self.primary_id = Some(primary_id.into());
                }
            }
            if let Some(mode) = value.get("sortMode").and_then(|v| v.as_str()).filter(|s| ["created", "near", "manual"].contains(s)) {
                self.list_sort = mode.into();
                self.event_page = 0;
            }
            if let Some(revision) = revision { self.revision = Some(self.revision.unwrap_or(0).max(revision)); }
            self.status = Some(if background_operation {
                StatusMessage::success("背景文件及引用已保存；请在腕端核实固件图片显示")
            } else if restore_operation && partial {
                StatusMessage::error("事件恢复已保存，但表盘文件同步失败；请重新同步列表，勿重复恢复")
            } else if restore_operation {
                StatusMessage::success("事件恢复已保存，请重新同步腕端列表")
            } else if primary_operation && partial {
                StatusMessage::error("表盘主事件已保存，但表盘文件同步失败；稍后打开腕端补写，无需重复设置")
            } else if primary_operation {
                StatusMessage::success("表盘主事件已保存，表盘文件已同步")
            } else if partial {
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
        self.result_status = self.status.clone();
        true
    }

    pub fn clear_selection(&mut self) {
        self.selected_event_key = None;
        self.pending_delete_key = None;
        self.modify_event_data = EventData::default();
    }

    pub fn primary_payload(&self, cancel: bool) -> Result<String, &'static str> {
        let event = self.selected_event_key.as_deref().and_then(|key| self.index_of_key(key))
            .and_then(|index| self.all_events.get(index)).ok_or("请先同步并选择事件")?;
        if event.id.is_empty() { return Err("表盘主事件需要稳定 ID，请更新腕端并同步"); }
        if cancel {
            if self.primary_id.as_deref() != Some(event.id.as_str()) { return Err("该事件不是当前表盘主事件，请重新同步核对"); }
        } else if event.extensions.get("archived").and_then(|v| v.as_bool()) == Some(true) {
            return Err("归档事件不能设为表盘主事件，请先恢复并同步");
        }
        Ok(crate::ui::protocol::set_primary(&event.id, cancel))
    }

    pub fn export_backup(&self) -> Result<serde_json::Value, &'static str> {
        if !self.snapshot_complete || self.snapshot_token.is_none() || self.snapshot_device != self.target_device {
            return Err("请先同步同设备的完整新列表，再导出或预览备份");
        }
        super::backup::make(&self.all_events, self.primary_id.as_deref().unwrap_or(""), &self.list_sort)
    }

    pub fn preview_backup(&mut self, backup: serde_json::Value) -> Result<(), &'static str> {
        self.export_backup()?;
        self.backup_preview = Some(super::backup::Preview::new(backup, &self.all_events,
            self.snapshot_device.clone().unwrap(), self.revision.unwrap())?);
        Ok(())
    }

    pub fn restore_payload(&self) -> Result<String, &'static str> {
        self.export_backup()?;
        let preview = self.backup_preview.as_ref().ok_or("请先选择备份并预览")?;
        if self.snapshot_device.as_deref() != Some(preview.device.as_str()) || self.revision != Some(preview.revision) {
            return Err("设备或数据已变化，请重新选择备份预览");
        }
        preview.payload()
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
        self.filtered_events().len().div_ceil(crate::ui::theme::EVENT_PAGE_SIZE).max(1)
    }

    /// 返回独立值，调用方释放读锁后才能进入发送/渲染路径。
    pub fn next_sort_mode(&self) -> String {
        match self.list_sort.as_str() { "created" => "near", "near" => "manual", _ => "created" }.into()
    }

    pub fn filtered_events(&self) -> Vec<(usize, &EventData)> {
        let mut events: Vec<_> = self.all_events.iter().enumerate().filter(|(_, e)| {
            e.extensions.get("archived").and_then(|v| v.as_bool()).unwrap_or(false) == self.list_archived &&
            (self.list_period == "all" || (!self.list_archived && self.period_matches(e))) &&
            (self.list_category == "all" || e.extensions.get("category").and_then(|v| v.as_str()).unwrap_or("uncategorized") == self.list_category)
        }).collect();
        events.sort_by_key(|(index, e)| {
            let pin = !e.extensions.get("pinned").and_then(|v| v.as_bool()).unwrap_or(false);
            let order = e.extensions.get("sortOrder").and_then(|v| v.as_u64()).unwrap_or(*index as u64);
            let occurrence = crate::ui::validation::event_occurrence(e, &self.today);
            let date = occurrence.map(|(d, _)| d).unwrap_or_else(|| e.time.clone());
            let past = date < self.today;
            let day = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d").ok().map(|d| d.num_days_from_ce()).unwrap_or(i32::MAX);
            let near = if past { -day } else { day };
            (pin, if self.list_sort == "near" { past } else { false },
                if self.list_sort == "near" { near as i64 } else if self.list_sort == "manual" { order as i64 } else { *index as i64 }, *index)
        });
        events
    }

    fn period_matches(&self, event: &EventData) -> bool {
        let Some((date, _)) = crate::ui::validation::event_occurrence(event, &self.today) else { return false; };
        let Some(days) = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d").ok().zip(
            chrono::NaiveDate::parse_from_str(&self.today, "%Y-%m-%d").ok()).map(|(date, today)| (date - today).num_days()) else { return false; };
        if self.list_period == "today" { days == 0 } else { (0..=30).contains(&days) }
    }

    pub fn page_events(&self) -> Vec<(usize, &EventData)> {
        let size = crate::ui::theme::EVENT_PAGE_SIZE;
        let page = self.event_page.min(self.max_page().saturating_sub(1));
        let start = page * size;
        self.filtered_events().into_iter().skip(start).take(size).collect()
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
    #[test]
    fn 排序取值后释放读锁才可预约发送() {
        let lock = std::sync::RwLock::new(super::UiState::new("2026-10-08".into()));
        for (current, next) in [("created", "near"), ("near", "manual"), ("manual", "created")] {
            lock.write().unwrap().list_sort = current.into();
            let mode = {
                let state = lock.read().unwrap();
                state.next_sort_mode()
            };
            let mut state = lock.try_write().expect("排序取值后不能保留读锁");
            assert_eq!(mode, next);
            assert!(state.reserve_send(serde_json::json!({"type":"setSortMode", "sortMode":mode}).to_string(), super::SendPurpose::Mutation).is_some());
            state.pending_send = None;
        }
    }
    #[test]
    fn 排序筛选有界分页保留原始身份且年度按下次日期() {
        let mut state = super::UiState::new("2026-10-08".into());
        for (id, date, repeat, pinned, archived, category) in [
            ("old", "2026-09-01", "none", false, false, "life"),
            ("future", "2026-10-10", "none", false, false, "birthday"),
            ("today", "2026-10-08", "none", false, false, "birthday"),
            ("annual", "2020-10-09", "yearly", false, false, "birthday"),
            ("pin", "2026-01-01", "none", true, false, "life"),
            ("archive", "2026-10-08", "none", false, true, "life"),
        ] {
            let mut event = super::EventData::draft(id, date, true, true);
            event.id = id.into();
            event.extensions = serde_json::from_value(serde_json::json!({"repeat":repeat,"pinned":pinned,"archived":archived,"category":category})).unwrap();
            state.all_events.push(event);
        }
        state.list_sort = "near".into();
        assert_eq!(state.page_events().iter().map(|(_, e)| e.id.as_str()).collect::<Vec<_>>(), ["pin", "today", "annual", "future", "old"]);
        state.list_category = "birthday".into();
        assert_eq!(state.page_events()[0].0, 2);
        assert!(state.open_editor("id:today"));
        assert_eq!(state.modify_event_data.id, "today");
        state.list_category = "all".into();
        state.list_archived = true;
        assert_eq!(state.page_events()[0].1.id, "archive");
    }
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
    fn 旧协议写操作须显式核实_刷新不自动解锁且草稿保留() {
        let (mut state, token) = waiting_result();
        let original = state.result_request.take().unwrap();
        state.begin_legacy_wait(&original);
        state.finish_send(token);
        state.event_data.name = "草稿".into();
        let fetch = state.reserve_send("{}".into(), SendPurpose::Fetch).unwrap();
        state.attach_send_device(fetch, "watch".into());
        let pending = state.claim_send(fetch).unwrap();
        state.begin_list_wait(&pending, false);
        assert!(state.accept_event_list(crate::ui::protocol::parse_event_list(r#"{"data":[]}"#).unwrap()));
        assert!(state.awaiting_confirmation);
        assert!(state.can_finish_unknown());
        assert!(state.finish_unknown_after_review());
        assert!(state.legacy_request.is_none());
        assert_eq!(state.event_data.name, "草稿");
    }

    #[test]
    fn 准备超时释放预约但不会清掉已发出的未知结果() {
        let mut state = UiState::new("2026-10-07".into());
        let token = state.reserve_send("{}".into(), SendPurpose::Mutation).unwrap();
        assert!(state.expire_preparation(token));
        assert!(!state.busy());
        assert!(!state.attach_send_device(token, "watch".into()));
        let (mut state, token) = waiting_result();
        assert!(!state.expire_preparation(token));
        assert!(state.busy());
    }

    #[test]
    fn 含无效跳过项的快照不能用来解除未知提交锁() {
        let (mut state, token) = waiting_result();
        state.finish_send(token);
        state.result_unknown = true;
        let fetch = state.reserve_send("{}".into(), SendPurpose::Fetch).unwrap();
        state.attach_send_device(fetch, "watch".into());
        let pending = state.claim_send(fetch).unwrap();
        state.begin_list_wait(&pending, true);
        let mut parsed = list_message(fetch, 11);
        parsed.skipped = 1;
        assert!(state.accept_event_list(parsed));
        assert!(!state.can_finish_unknown());
        assert!(state.awaiting_confirmation);
    }

    #[test]
    fn 查询无响应转待核实_旧查询timer不影响新查询或已结算结果() {
        let (mut state, token) = waiting_result();
        state.finish_send(token);
        let (old, _) = state.begin_result_query().unwrap();
        let (current, _) = state.begin_result_query().unwrap();
        assert!(!state.expire_result_query(old));
        assert!(state.expire_result_query(current));
        assert!(state.result_unknown);
        assert!(state.awaiting_confirmation);
        assert!(!state.can_finish_unknown());
        let (query, _) = state.begin_result_query().unwrap();
        assert!(state.apply_request_message(&result_message(token, true)));
        assert!(!state.expire_result_query(query));
        state.status = Some(StatusMessage::info("正在刷新"));
        assert_eq!(crate::ui::view::build(&state).status.unwrap().text, "腕端已保存");
    }

    #[test]
    fn 原请求未结算不能换设备核实_回到原设备仍可刷新() {
        let (mut state, token) = waiting_result();
        state.finish_send(token);
        let fetch = state.reserve_send("{}".into(), SendPurpose::Fetch).unwrap();
        assert!(!state.attach_send_device(fetch, "other".into()));
        assert_eq!(state.target_device.as_deref(), Some("watch"));
        assert!(state.result_request.is_some());
        assert!(state.attach_send_device(fetch, "watch".into()));
    }

    #[test]
    fn 同会话旧握手和错误设备不得提前完成新握手() {
        let (mut state, token) = waiting_result();
        state.start_handshake(token);
        let mut reply = serde_json::json!({"sessionId":"session", "deviceId":"watch",
            "handshakeId":"hello-old", "protocolVersion":2,"capabilities":["requestId"]});
        assert!(!state.accept_capabilities(&reply));
        reply["handshakeId"] = serde_json::json!(format!("hello-{token}"));
        reply["deviceId"] = serde_json::json!("other");
        assert!(!state.accept_capabilities(&reply));
        reply["deviceId"] = serde_json::json!("watch");
        assert!(state.accept_capabilities(&reply));
        assert!(state.supports("requestId"));
    }

    #[test]
    fn 未知结果须完整新快照及显式核实才解锁_迟到回执不串新请求() {
        let (mut state, original) = waiting_result();
        state.finish_send(original);
        state.event_data.name = "草稿".into();
        let mut status = result_message(original, true);
        status["type"] = serde_json::json!("requestStatus");
        status["status"] = serde_json::json!("unknown");
        assert!(state.apply_request_message(&status));
        assert!(!state.finish_unknown_after_review());
        let token = state.reserve_send("{}".into(), SendPurpose::Fetch).unwrap();
        state.attach_send_device(token, "watch".into());
        let pending = state.claim_send(token).unwrap();
        state.begin_list_wait(&pending, true);
        let mut parsed = list_message(token, 11);
        parsed.request_id = Some(pending.request_id.clone());
        assert!(state.accept_event_list(parsed));
        assert!(state.awaiting_confirmation, "刷新不能自动解锁");
        assert!(state.can_finish_unknown());
        let view = crate::ui::view::build(&state);
        assert_eq!(view.secondary.as_ref().unwrap().id, crate::ui::dispatch::UiId::FinishUnknown);
        assert!(!view.primary.as_ref().unwrap().enabled);
        assert_eq!(crate::ui::dispatch::resolve("action.result.reviewed", crate::ui::dispatch::UiEventKind::Click, ""),
            Some(crate::ui::dispatch::Action::FinishUnknown));
        assert!(state.finish_unknown_after_review());
        assert_eq!(state.event_data.name, "草稿");
        assert!(!state.busy());
        let newer = state.reserve_send("new".into(), SendPurpose::Mutation).unwrap();
        assert!(!state.apply_request_message(&result_message(original, true)));
        assert_eq!(state.pending_send.as_ref().unwrap().token, newer);
    }

    #[test]
    fn 处理中及过期核实快照不能解除未知锁() {
        let (mut state, token) = waiting_result();
        state.finish_send(token);
        state.result_unknown = true;
        state.snapshot_device = Some("watch".into());
        state.snapshot_token = Some(9);
        state.recovery_snapshot = Some(9);
        assert!(state.can_finish_unknown());
        state.expire_snapshot(9);
        assert!(!state.finish_unknown_after_review());
        state.snapshot_token = Some(10);
        state.recovery_snapshot = Some(10);
        let mut status = result_message(token, true);
        status["type"] = serde_json::json!("requestStatus");
        status["status"] = serde_json::json!("processing");
        state.apply_request_message(&status);
        assert!(!state.finish_unknown_after_review());
        assert!(state.awaiting_confirmation);
    }

    fn waiting_list(identified: bool) -> (UiState, u64) {
        let mut state = state_with_events();
        state.target_device = Some("watch".into());
        state.session_id = "session".into();
        state.revision = Some(10);
        let token = state.reserve_send(r#"{"type":"getAllEvent"}"#.into(), SendPurpose::Fetch).unwrap();
        state.attach_send_device(token, "watch".into());
        let pending = state.claim_send(token).unwrap();
        state.begin_list_wait(&pending, identified);
        (state, token)
    }

    #[test]
    fn 旧下标刷新身份变化撤销选择但保留修改草稿() {
        let (mut state, token) = waiting_list(false);
        state.all_events = vec![EventData::draft("旧事件", "2026-01-01", false, false)];
        state.select_event("idx:0");
        state.modify_event_data.name = "未提交".into();
        let parsed = crate::ui::protocol::parse_event_list(r#"{"data":[{"name":"另一个事件","date":"2026-01-01"}]}"#).unwrap();
        assert!(state.accept_event_list(parsed));
        assert!(state.selected_event_key.is_none());
        assert_eq!(state.modify_event_data.name, "未提交");
        assert_eq!(state.snapshot_token, Some(token));
    }

    fn batch_message(token: u64, index: usize, name: &str) -> serde_json::Value {
        serde_json::json!({"type":"eventListBatch", "requestId":format!("req-{token}"),
            "sessionId":"session", "deviceId":"watch", "revision":11, "primaryId":"",
            "total":2, "batchCount":2, "batchIndex":index,
            "data":[{"name":name,"date":"2026-01-01"}]})
    }

    #[test]
    fn 分批乱序重复仅在齐全后提交_原始下标和草稿保留() {
        let (mut state, token) = waiting_list(true);
        state.event_data.name = "草稿".into();
        let second = batch_message(token, 1, "乙 🎂");
        assert!(!state.accept_list_batch(&second));
        assert!(!state.accept_list_batch(&second));
        assert_eq!(state.all_events.len(), 3);
        assert!(state.accept_list_batch(&batch_message(token, 0, "甲\"\\\n")));
        assert_eq!(state.all_events[0].name, "甲\"\\\n");
        assert_eq!(state.all_events[1].name, "乙 🎂");
        assert_eq!(state.all_events[1].selection_key(0), "idx:1");
        assert_eq!(state.event_data.name, "草稿");
        assert!(state.list_assembly.is_none());
        assert!(!state.accept_list_batch(&second));
    }

    #[test]
    fn 分批冲突元数据数量与预算失败均保留旧列表() {
        for issue in ["conflict", "revision", "total", "size", "count"] {
            let (mut state, token) = waiting_list(true);
            assert!(!state.accept_list_batch(&batch_message(token, 0, "甲")));
            let mut next = batch_message(token, 1, "乙");
            match issue {
                "conflict" => next = batch_message(token, 0, "冲突"),
                "revision" => next["revision"] = serde_json::json!(12),
                "total" => next["total"] = serde_json::json!(3),
                "size" => next["data"][0]["name"] = serde_json::json!("x".repeat(8192)),
                _ => next["batchCount"] = serde_json::json!(201),
            }
            assert!(state.accept_list_batch(&next));
            assert_eq!(state.all_events.len(), 3);
            assert!(state.list_request.is_none());
            assert!(state.list_assembly.is_none());
        }
    }

    #[test]
    fn 缺批超时与旧请求失败不影响新请求_空列表可提交() {
        let (mut state, token) = waiting_list(true);
        state.accept_list_batch(&batch_message(token, 0, "甲"));
        assert!(!state.accept_list_batch(&batch_message(token + 1, 1, "旧")));
        assert!(state.expire_list(token));
        assert!(state.list_assembly.is_none());
        assert_eq!(state.all_events.len(), 3);
        let (mut state, token) = waiting_list(true);
        let mut empty = batch_message(token, 0, "");
        empty["data"] = serde_json::json!([]);
        empty["total"] = serde_json::json!(0);
        empty["batchCount"] = serde_json::json!(1);
        assert!(state.accept_list_batch(&empty));
        assert!(state.all_events.is_empty());
    }

    #[test]
    fn 旧下标快照过期拒绝写操作_旧timer不影响新快照() {
        let (mut state, token) = waiting_list(false);
        let parsed = crate::ui::protocol::parse_event_list(r#"{"data":[null,{"name":"旧端","date":"2026-01-01"}]}"#).unwrap();
        assert!(state.accept_event_list(parsed));
        let operation = state.reserve_send(r#"{"type":"deleteEvent","index":1}"#.into(), SendPurpose::Mutation).unwrap();
        assert!(state.attach_send_device(operation, "watch".into()));
        state.finish_send(operation);
        state.expire_snapshot(token + 1);
        assert_eq!(state.snapshot_token, Some(token));
        state.expire_snapshot(token);
        let operation = state.reserve_send(r#"{"type":"deleteEvent","index":1}"#.into(), SendPurpose::Mutation).unwrap();
        assert!(!state.attach_send_device(operation, "watch".into()));
    }

    fn list_message(token: u64, revision: u64) -> crate::ui::protocol::ParsedEvents {
        crate::ui::protocol::parse_event_list(&serde_json::json!({"data":[{"id":"e1", "name":"远端", "date":"2026-12-01"}],
            "requestId":format!("req-{token}"), "sessionId":"session", "deviceId":"watch", "revision":revision}).to_string()).unwrap()
    }

    #[test]
    fn 列表身份校验拒绝旧请求会话设备及revision_拒绝不改快照() {
        let (mut state, token) = waiting_list(true);
        for field in ["request", "session", "device", "revision"] {
            let mut parsed = list_message(token, 11);
            match field {
                "request" => parsed.request_id = Some("old".into()),
                "session" => parsed.session_id = Some("old".into()),
                "device" => parsed.device_id = Some("other".into()),
                _ => parsed.revision = Some(9),
            }
            assert!(!state.accept_event_list(parsed));
            assert_eq!(state.all_events.len(), 3);
            assert!(state.list_request.is_some());
        }
        assert!(state.accept_event_list(list_message(token, 11)));
        assert!(state.finish_send(token).is_none(), "早到列表消费发送token");
        assert!(!state.accept_event_list(list_message(token, 12)), "重复回包不结算");
        assert_eq!(state.revision, Some(11));
    }

    #[test]
    fn 新拉取替代旧请求_超时后迟到列表不能更新() {
        let (mut state, old) = waiting_list(true);
        state.finish_send(old);
        let token = state.reserve_send("{}".into(), SendPurpose::Fetch).unwrap();
        state.attach_send_device(token, "watch".into());
        let pending = state.claim_send(token).unwrap();
        state.begin_list_wait(&pending, true);
        assert!(!state.accept_event_list(list_message(old, 11)));
        assert!(!state.expire_list(old));
        assert!(state.expire_list(token));
        assert!(!state.accept_event_list(list_message(token, 11)));
        assert_eq!(state.all_events.len(), 3);
    }

    #[test]
    fn 刷新保留新增修改草稿_选择失效不清空未提交表单() {
        let (mut state, token) = waiting_list(true);
        state.select_event("id:e2");
        state.modify_event_data.name = "未提交修改".into();
        state.event_data.name = "未提交新增".into();
        assert!(state.accept_event_list(list_message(token, 11)));
        assert!(state.selected_event_key.is_none());
        assert_eq!(state.modify_event_data.name, "未提交修改");
        assert_eq!(state.event_data.name, "未提交新增");
        assert_eq!(state.snapshot_device.as_deref(), Some("watch"));
    }

    #[test]
    fn 旧端列表仅在拉取窗口接收_新协议不接受无身份列表() {
        let old = r#"{"data":[{"name":"旧端","date":"2026-01-01"}]}"#;
        let (mut state, _) = waiting_list(true);
        assert!(!state.accept_event_list(crate::ui::protocol::parse_event_list(old).unwrap()));
        let (mut state, token) = waiting_list(false);
        assert!(state.accept_event_list(crate::ui::protocol::parse_event_list(old).unwrap()));
        assert!(!state.accept_event_list(crate::ui::protocol::parse_event_list(old).unwrap()));
        assert!(!state.expire_list(token));
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
    fn 屏幕尺寸仅接受本次设备握手并使旧预览失效() {
        let (mut state, token) = waiting_result();
        state.start_handshake(token);
        state.background_preview = Some("old".into());
        let mut reply = serde_json::json!({"sessionId":"session","protocolVersion":2,"handshakeId":format!("hello-{token}"),
            "deviceId":"other","screenWidth":336,"screenHeight":480,"capabilities":[]});
        assert!(!state.accept_capabilities(&reply));
        assert_eq!(state.background_width,"192");
        reply["deviceId"] = serde_json::json!("watch");
        assert!(state.accept_capabilities(&reply));
        assert_eq!(state.background_width,"336");
        assert_eq!(state.background_height,"480");
        assert_eq!(state.background_device.as_deref(),Some("watch"));
        assert!(state.background_preview.is_none());
    }

    #[test]
    fn 首次握手等待后按逐项能力启用_旧会话与迟到回包不覆盖() {
        let (mut state, token) = waiting_result();
        state.start_handshake(token);
        assert!(!state.supports("requestId"));
        assert!(state.poll_handshake(token, 8));
        let mut capabilities = serde_json::json!({"sessionId":"stale", "protocolVersion":2,
            "handshakeId":format!("hello-{token}"), "deviceId":"watch",
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
