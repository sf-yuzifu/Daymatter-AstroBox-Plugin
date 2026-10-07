//! 事件标识与动作派发：单一命名表驱动 UI 绑定与处理分支（E-16 / E-18）。
//!
//! 命名规则：`{区域}.{scope}.{角色}`，例如 `field.modify.on_home.yes`、
//! `action.delete.submit`、`pick.modify.id:e1`。所有绑定只从这里取字符串，
//! 所有处理分支只从 `resolve` 出去，因此「常量 ↔ 分支」可以在 host 上双向核对。

use crate::ui::state::{EventType, FormScope, StatusMessage, UiState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldRole {
    Name,
    Date,
    OnHome,
    CountStartDay,
}

impl FieldRole {
    pub fn as_str(self) -> &'static str {
        match self {
            FieldRole::Name => "name",
            FieldRole::Date => "date",
            FieldRole::OnHome => "on_home",
            FieldRole::CountStartDay => "count_start",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            FieldRole::Name => "事件名称",
            FieldRole::Date => "目标日期",
            FieldRole::OnHome => "显示在快应用首页",
            FieldRole::CountStartDay => "计入起始日",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            FieldRole::Name => "最多 50 个字符",
            FieldRole::Date => "格式 YYYY-MM-DD，需为真实日期",
            FieldRole::OnHome => "只影响快应用首页卡片，与表盘无关",
            FieldRole::CountStartDay => "开启后目标日本身算第 1 天",
        }
    }

    pub const ALL: [FieldRole; 4] = [
        FieldRole::Name,
        FieldRole::Date,
        FieldRole::OnHome,
        FieldRole::CountStartDay,
    ];
}

/// 预留字段位：F-01 年度重复 / F-02 排序置顶 / F-03 表盘主事件落地时只需在这里加行（E-20）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservedRole {
    Repeat,
    Sort,
    Pinned,
    Watchface,
}

impl ReservedRole {
    pub fn as_str(self) -> &'static str {
        match self {
            ReservedRole::Repeat => "repeat",
            ReservedRole::Sort => "sort",
            ReservedRole::Pinned => "pinned",
            ReservedRole::Watchface => "watchface",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ReservedRole::Repeat => "每年重复",
            ReservedRole::Sort => "排序",
            ReservedRole::Pinned => "置顶",
            ReservedRole::Watchface => "表盘主事件",
        }
    }

    pub const ALL: [ReservedRole; 4] = [
        ReservedRole::Repeat,
        ReservedRole::Sort,
        ReservedRole::Pinned,
        ReservedRole::Watchface,
    ];
}

/// 静态元素标识：与 `UiId::ALL` 一一对应
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiId {
    Segment(EventType),
    Field(FormScope, FieldRole),
    SwitchOn(FormScope, FieldRole),
    SwitchOff(FormScope, FieldRole),
    Submit(FormScope),
    Fetch(FormScope),
    PagePrev(FormScope),
    PageNext(FormScope),
    PickBackground,
    StatusDismiss,
    BackOverview,
    FinishUnknown,
}

impl UiId {
    pub const ALL: [UiId; 33] = [
        UiId::FinishUnknown,
        UiId::Segment(EventType::AddEvent),
        UiId::Segment(EventType::ModifyEvent),
        UiId::Segment(EventType::DeleteEvent),
        UiId::Segment(EventType::AddBackground),
        UiId::Field(FormScope::Add, FieldRole::Name),
        UiId::Field(FormScope::Add, FieldRole::Date),
        UiId::Field(FormScope::Add, FieldRole::OnHome),
        UiId::Field(FormScope::Add, FieldRole::CountStartDay),
        UiId::Field(FormScope::Modify, FieldRole::Name),
        UiId::Field(FormScope::Modify, FieldRole::Date),
        UiId::Field(FormScope::Modify, FieldRole::OnHome),
        UiId::Field(FormScope::Modify, FieldRole::CountStartDay),
        UiId::SwitchOn(FormScope::Add, FieldRole::OnHome),
        UiId::SwitchOff(FormScope::Add, FieldRole::OnHome),
        UiId::SwitchOn(FormScope::Add, FieldRole::CountStartDay),
        UiId::SwitchOff(FormScope::Add, FieldRole::CountStartDay),
        UiId::SwitchOn(FormScope::Modify, FieldRole::OnHome),
        UiId::SwitchOff(FormScope::Modify, FieldRole::OnHome),
        UiId::SwitchOn(FormScope::Modify, FieldRole::CountStartDay),
        UiId::SwitchOff(FormScope::Modify, FieldRole::CountStartDay),
        UiId::BackOverview,
        UiId::Submit(FormScope::Add),
        UiId::Submit(FormScope::Modify),
        UiId::Submit(FormScope::Delete),
        UiId::Fetch(FormScope::Modify),
        UiId::Fetch(FormScope::Delete),
        UiId::PagePrev(FormScope::Modify),
        UiId::PageNext(FormScope::Modify),
        UiId::PagePrev(FormScope::Delete),
        UiId::PageNext(FormScope::Delete),
        UiId::PickBackground,
        UiId::StatusDismiss,
    ];

    pub fn as_str(self) -> String {
        match self {
            UiId::Segment(tab) => format!("segment.{}", tab_scope(tab)),
            UiId::Field(scope, role) => format!("field.{}.{}", scope.as_str(), role.as_str()),
            UiId::SwitchOn(scope, role) => format!("field.{}.{}.on", scope.as_str(), role.as_str()),
            UiId::SwitchOff(scope, role) => format!("field.{}.{}.off", scope.as_str(), role.as_str()),
            UiId::Submit(scope) => format!("action.{}.submit", scope.as_str()),
            UiId::Fetch(scope) => format!("action.{}.fetch", scope.as_str()),
            UiId::PagePrev(scope) => format!("action.{}.page.prev", scope.as_str()),
            UiId::PageNext(scope) => format!("action.{}.page.next", scope.as_str()),
            UiId::PickBackground => "action.background.pick".to_string(),
            UiId::BackOverview => "nav.back".to_string(),
            UiId::StatusDismiss => "status.dismiss".to_string(),
            UiId::FinishUnknown => "action.result.reviewed".to_string(),
        }
    }

    pub fn parse(id: &str) -> Option<UiId> {
        UiId::ALL.into_iter().find(|candidate| candidate.as_str() == id)
    }
}

fn tab_scope(tab: EventType) -> &'static str {
    match tab {
        EventType::AddEvent => "add",
        EventType::ModifyEvent => "modify",
        EventType::DeleteEvent => "delete",
        EventType::AddBackground => "background",
    }
}

fn scope_from_str(text: &str) -> Option<FormScope> {
    match text {
        "add" => Some(FormScope::Add),
        "modify" => Some(FormScope::Modify),
        "delete" => Some(FormScope::Delete),
        _ => None,
    }
}

/// 事件列表行标识：`pick.{scope}.{key}`，key 为稳定 ID 或旧数据下标键（E-11）
pub fn pick_id(scope: FormScope, key: &str) -> String {
    format!("pick.{}.{}", scope.as_str(), key)
}

pub fn parse_pick(id: &str) -> Option<(FormScope, String)> {
    let rest = id.strip_prefix("pick.")?;
    let (scope, key) = rest.split_once('.')?;
    Some((scope_from_str(scope)?, key.to_string()))
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    SwitchSegment(EventType),
    BackOverview,
    SetText { scope: FormScope, role: FieldRole, value: String },
    SetSwitch { scope: FormScope, role: FieldRole, on: bool },
    PickEvent { scope: FormScope, key: String },
    PageStep { scope: FormScope, delta: isize },
    SubmitAdd,
    SubmitModify,
    FetchEvents,
    RequestDelete,
    ConfirmDelete,
    CancelDelete,
    PickBackground,
    DismissStatus,
    FinishUnknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiEventKind {
    Click,
    Change,
}

impl UiEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            UiEventKind::Click => "click",
            UiEventKind::Change => "change",
        }
    }
}

/// 从宿主载荷里取文本：兼容 `{"value":"..."}` 与裸字符串
pub fn payload_text(payload: &str) -> String {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(payload) {
        if let Some(text) = json.get("value").and_then(|value| value.as_str()) {
            return text.to_string();
        }
    }
    payload.to_string()
}

/// 事件标识 → 动作；`None` 表示该标识当前不应产生任何状态变化
pub fn resolve(id: &str, kind: UiEventKind, payload: &str) -> Option<Action> {
    if let Some((scope, key)) = parse_pick(id) {
        return match kind {
            UiEventKind::Click => Some(Action::PickEvent { scope, key }),
            UiEventKind::Change => None,
        };
    }
    let ui_id = UiId::parse(id)?;
    let click = kind == UiEventKind::Click;
    Some(match ui_id {
        UiId::Segment(tab) if click => Action::SwitchSegment(tab),
        UiId::BackOverview if click => Action::BackOverview,
        UiId::Field(scope, role) if !click => Action::SetText {
            scope,
            role,
            value: payload_text(payload),
        },
        UiId::SwitchOn(scope, role) if click => Action::SetSwitch { scope, role, on: true },
        UiId::SwitchOff(scope, role) if click => Action::SetSwitch { scope, role, on: false },
        UiId::Submit(FormScope::Add) if click => Action::SubmitAdd,
        UiId::Submit(FormScope::Modify) if click => Action::SubmitModify,
        UiId::Submit(FormScope::Delete) if click => Action::RequestDelete,
        UiId::Fetch(_) if click => Action::FetchEvents,
        UiId::PagePrev(scope) if click => Action::PageStep { scope, delta: -1 },
        UiId::PageNext(scope) if click => Action::PageStep { scope, delta: 1 },
        UiId::PickBackground if click => Action::PickBackground,
        UiId::StatusDismiss if click => Action::DismissStatus,
        UiId::FinishUnknown if click => Action::FinishUnknown,
        _ => return None,
    })
}

/// 纯状态变换：动作 → 新状态 + 是否需要重建界面（E-17 去重依据）
pub struct StateChange {
    pub action: Action,
    pub rebuild: bool,
    pub status: Option<StatusMessage>,
}

pub fn apply_action(state: &mut UiState, action: Action) -> StateChange {
    let mut rebuild = true;
    let mut status = None;
    if state.delete_dialog_open && !matches!(action, Action::DismissStatus) {
        return StateChange { action, rebuild: false, status: None };
    }
    // 在途锁定：正在与腕端交互时忽略会重复派发的动作（P1-51）
    if state.busy()
        && matches!(
            action,
            Action::SubmitAdd | Action::SubmitModify | Action::RequestDelete | Action::PickBackground
        )
    {
        return StateChange { action, rebuild: false, status: None };
    }
    match action.clone() {
        Action::FinishUnknown => { rebuild = state.finish_unknown_after_review(); }
        Action::SwitchSegment(tab) => {
            if state.current_tab == tab && !state.route.is_editor() {
                rebuild = false;
            } else {
                state.current_tab = tab;
                state.route = crate::ui::state::Route::Overview;
                state.pending_delete_key = None;
            }
        }
        Action::BackOverview => {
            if !state.route.is_editor() {
                rebuild = false;
            }
            state.back_to_overview();
        }
        Action::SetText { scope, role, value } => {
            if let Some(draft) = state.draft_mut(scope) {
                match role {
                    FieldRole::Name => draft.name = value,
                    FieldRole::Date => draft.time = value,
                    _ => {}
                }
            }
            // 输入只改草稿，不清状态条，避免用户刚看到的错误提示被输入抹掉
            rebuild = false;
        }
        Action::SetSwitch { scope, role, on } => {
            if let Some(draft) = state.draft_mut(scope) {
                match role {
                    FieldRole::OnHome => draft.on_index = on,
                    FieldRole::CountStartDay => draft.if_staring_day = on,
                    _ => {}
                }
            }
        }
        Action::PickEvent { scope, key } => {
            // 修改页进入独立编辑子页，删除页原地选中
            let ok = if scope == FormScope::Modify {
                state.open_editor(&key)
            } else {
                let ok = state.select_event(&key);
                state.pending_delete_key = if ok { Some(key.clone()) } else { None };
                ok
            };
            if !ok {
                status = Some(StatusMessage::error("事件列表已变化，请重新获取后选择"));
            }
        }
        Action::PageStep { scope, delta } => {
            let _ = scope;
            let max_page = state.max_page() as isize;
            let next = (state.event_page as isize + delta).clamp(0, max_page - 1);
            rebuild = next != state.event_page as isize;
            state.event_page = next as usize;
        }
        Action::DismissStatus => {
            if state.status.is_none() {
                rebuild = false;
            }
            state.status = None;
        }
        Action::FetchEvents => {
            status = Some(StatusMessage::info(SUBMITTING_TEXT));
        }
        Action::SubmitAdd | Action::SubmitModify => {
            status = Some(StatusMessage::info(SUBMITTING_TEXT));
        }
        Action::RequestDelete => {
            if state.selected_event_key.is_none() {
                status = Some(StatusMessage::error(NO_SELECTION_TEXT));
            } else {
                state.pending_delete_key = state.selected_event_key.clone();
            }
        }
        Action::ConfirmDelete | Action::CancelDelete => {
            state.pending_delete_key = None;
            rebuild = false;
        }
        Action::PickBackground => {
            status = Some(StatusMessage::info("正在选择图片，请稍等…"));
        }
    }
    if let Some(message) = &status {
        state.status = Some(message.clone());
    }
    StateChange { action, rebuild, status }
}

pub const SUBMITTING_TEXT: &str = "正在发送，请稍等…";
pub const FETCH_SUCCESS_TEXT: &str = "已获取手环端数据";
pub const NO_DEVICE_TEXT: &str = "未找到已连接设备";
pub const APP_MISSING_TEXT: &str = "未找到倒数日快应用";
pub const APP_TOO_OLD_TEXT: &str = "请先升级倒数日快应用到 2.1 及以上";
pub const SEND_FAILED_TEXT: &str = "发送失败，请重试";
pub const SENT_TEXT: &str = "已发送，等待腕端确认";
pub const NAME_REQUIRED_TEXT: &str = "事件名称不能为空";
pub const NO_SELECTION_TEXT: &str = "请先选择事件";
pub const CONFIRM_DELETE_TITLE: &str = "确认删除";
pub const CONFIRM_DELETE_ACCEPT: &str = "delete_accept";
pub const CONFIRM_DELETE_CANCEL: &str = "delete_cancel";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::state::{EventData, StatusKind};

    fn ids() -> Vec<String> {
        UiId::ALL.iter().map(|id| id.as_str()).collect()
    }

    #[test]
    fn 标识表无重复且可反解() {
        let all = ids();
        let mut sorted = all.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), all.len(), "事件标识必须唯一");
        for id in &all {
            assert_eq!(UiId::parse(id).map(|parsed| parsed.as_str()), Some(id.clone()), "标识必须可反解");
        }
    }

    #[test]
    fn 文本字段只响应变更_其余标识只响应点击() {
        for id in UiId::ALL {
            let click = resolve(&id.as_str(), UiEventKind::Click, "");
            let change = resolve(&id.as_str(), UiEventKind::Change, r#"{"value":"x"}"#);
            match id {
                UiId::Field(_, _) => {
                    assert!(click.is_none(), "{} 是文本框，不应有点击分支", id.as_str());
                    assert!(change.is_some(), "{} 缺少变更分支", id.as_str());
                }
                _ => {
                    assert!(click.is_some(), "{} 缺少点击分支", id.as_str());
                    assert!(change.is_none(), "{} 不应响应 change", id.as_str());
                }
            }
        }
    }

    #[test]
    fn 文本字段在变更事件下有动作分支() {
        for scope in [FormScope::Add, FormScope::Modify] {
            for role in FieldRole::ALL {
                let id = UiId::Field(scope, role).as_str();
                let action = resolve(&id, UiEventKind::Change, r#"{"value":"2026-01-01"}"#).expect("文本字段分支");
                match action {
                    Action::SetText { role: parsed, value, .. } => {
                        assert_eq!(parsed, role);
                        assert_eq!(value, "2026-01-01");
                    }
                    other => panic!("{} 派发错误: {:?}", id, other),
                }
            }
        }
    }

    #[test]
    fn 非文本标识在变更事件下不产生动作() {
        for id in [UiId::Segment(EventType::AddEvent).as_str(), UiId::Submit(FormScope::Add).as_str()] {
            assert!(resolve(&id, UiEventKind::Change, "{}").is_none(), "{} 不应响应 change", id);
        }
    }

    #[test]
    fn 返回按钮只响应点击() {
        let id = UiId::BackOverview.as_str();
        assert_eq!(id, "nav.back");
        assert_eq!(resolve(&id, UiEventKind::Click, ""), Some(Action::BackOverview));
        assert!(resolve(&id, UiEventKind::Change, "").is_none());
    }

    #[test]
    fn 列表行标识三个scope都可解析() {
        for scope in [FormScope::Add, FormScope::Modify, FormScope::Delete] {
            let id = pick_id(scope, "id:e1");
            assert_eq!(parse_pick(&id), Some((scope, "id:e1".to_string())));
        }
    }

    #[test]
    fn 列表行标识可解析出scope与稳定键() {
        let id = pick_id(FormScope::Modify, "id:e1");
        assert_eq!(parse_pick(&id), Some((FormScope::Modify, "id:e1".to_string())));
        assert!(resolve(&id, UiEventKind::Click, "").is_some());
        assert!(parse_pick("pick.unknown.x").is_none());
        assert!(parse_pick("field.add.name").is_none());
    }

    #[test]
    fn 开关动作只改对应字段() {
        let mut state = UiState::new("2026-10-06".to_string());
        apply_action(&mut state, Action::SetSwitch {
            scope: FormScope::Add,
            role: FieldRole::OnHome,
            on: true,
        });
        assert!(state.event_data.on_index);
        assert!(!state.event_data.if_staring_day, "不得影响其它开关");
        apply_action(&mut state, Action::SetSwitch {
            scope: FormScope::Modify,
            role: FieldRole::CountStartDay,
            on: true,
        });
        assert!(state.modify_event_data.if_staring_day);
        assert!(!state.modify_event_data.on_index);
    }

    #[test]
    fn 输入不触发重建也不清状态提示() {
        let mut state = UiState::new("2026-10-06".to_string());
        state.status = Some(StatusMessage::error("日期无效"));
        let change = apply_action(
            &mut state,
            Action::SetText { scope: FormScope::Add, role: FieldRole::Name, value: "生日".into() },
        );
        assert!(!change.rebuild, "输入只改草稿，不应重建界面");
        assert_eq!(state.event_data.name, "生日");
        assert!(state.status.is_some(), "输入不得抹掉状态提示");
    }

    #[test]
    fn 重复点击当前段不重建() {
        let mut state = UiState::new("2026-10-06".to_string());
        let change = apply_action(&mut state, Action::SwitchSegment(EventType::AddEvent));
        assert!(!change.rebuild);
        let change = apply_action(&mut state, Action::SwitchSegment(EventType::ModifyEvent));
        assert!(change.rebuild);
        assert_eq!(state.current_tab, EventType::ModifyEvent);
    }

    #[test]
    fn 子页返回概览且概览下返回无效() {
        let mut state = UiState::new("2026-10-06".to_string());
        state.route = crate::ui::state::Route::Editor(FormScope::Modify);
        let change = apply_action(&mut state, Action::BackOverview);
        assert!(change.rebuild);
        assert_eq!(state.route, crate::ui::state::Route::Overview);
        let change = apply_action(&mut state, Action::BackOverview);
        assert!(!change.rebuild, "概览页再返回不重建");
    }

    #[test]
    fn 页码在边界不重建() {
        let mut state = UiState::new("2026-10-06".to_string());
        let change = apply_action(&mut state, Action::PageStep { scope: FormScope::Modify, delta: -1 });
        assert!(!change.rebuild, "首页再往前不应重建");
        state.all_events = (0..10)
            .map(|index| EventData {
                id: format!("e{}", index),
                name: format!("事件{}", index),
                time: "2026-01-01".into(),
                ..Default::default()
            })
            .collect();
        let change = apply_action(&mut state, Action::PageStep { scope: FormScope::Modify, delta: 1 });
        assert!(change.rebuild);
        assert_eq!(state.event_page, 1);
        let change = apply_action(&mut state, Action::PageStep { scope: FormScope::Modify, delta: 1 });
        assert!(!change.rebuild, "末页再往后不重建");
    }

    #[test]
    fn 选择失效给出提示且不改草稿() {
        let mut state = UiState::new("2026-10-06".to_string());
        state.all_events.push(EventData {
            id: "e1".into(),
            name: "旧事件".into(),
            time: "2026-01-01".into(),
            ..Default::default()
        });
        state.select_event("id:e1");
        let change = apply_action(
            &mut state,
            Action::PickEvent { scope: FormScope::Modify, key: "id:已删除".into() },
        );
        assert_eq!(change.status.as_ref().map(|s| s.kind), Some(StatusKind::Error));
        assert_eq!(state.modify_event_data.name, "旧事件");
    }

    #[test]
    fn 预留字段尚未接入绑定() {
        for role in ReservedRole::ALL {
            for scope in [FormScope::Add, FormScope::Modify] {
                let id = format!("field.{}.{}", scope.as_str(), role.as_str());
                assert!(UiId::parse(&id).is_none(), "{} 尚未实现，不应有绑定", id);
            }
        }
    }
}
