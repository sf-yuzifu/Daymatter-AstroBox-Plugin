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
        });
        Some(token)
    }

    pub fn attach_send_device(&mut self, token: u64, device_addr: String) -> bool {
        let Some(pending) = self.pending_send.as_mut().filter(|pending| pending.token == token) else {
            return false;
        };
        pending.device_addr = device_addr;
        pending.in_progress = false;
        true
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
        self.pending_send.take()
    }

    pub fn apply_confirmed_list(&mut self, primary_id: Option<String>, revision: Option<u64>) {
        if self.pending_send.as_ref().is_some_and(|pending| pending.purpose == SendPurpose::Fetch) {
            // The list can arrive before the send-transport callback. Consume
            // the fetch token here so its later timer completion is harmless.
            self.pending_send = None;
        }
        if self.awaiting_confirmation {
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
