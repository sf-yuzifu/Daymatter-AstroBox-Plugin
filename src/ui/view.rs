//! 视图模型：由状态推导出的 ClaraLight 结构化描述（分段控制器 / 常驻状态面板 /
//! 概览卡片 / 独立编辑子页 / 单一主操作），宿主渲染层只负责翻译成元素树。
//!
//! 结构与文案都在这里决策，因此可在 host 上断言；界面重建前用指纹比对，内容没变不重画。

use crate::ui::dispatch::{pick_id, FieldRole, UiId};
use crate::ui::state::{EventType, FormScope, Route, StatusKind, UiState};
use crate::ui::theme::ButtonKind;

#[derive(Debug, Clone, PartialEq)]
pub enum FieldView {
    Text { id: UiId, value: String, label: &'static str, hint: &'static str },
    Select { id: UiId, value: String, options: Vec<(&'static str, &'static str)>, label: &'static str, hint: &'static str },
    Switch { on_id: UiId, off_id: UiId, on: bool, label: &'static str, hint: &'static str },
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventRowView {
    pub id: String,
    pub label: String,
    pub detail: String,
    pub selected: bool,
    pub navigates: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventListView {
    pub scope: FormScope,
    pub title: &'static str,
    pub description: &'static str,
    pub rows: Vec<EventRowView>,
    pub page: usize,
    pub max_page: usize,
    pub total: usize,
    pub prev_id: UiId,
    pub next_id: UiId,
    pub footnote: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionView {
    pub id: UiId,
    pub label: String,
    pub kind: ButtonKind,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SegmentView {
    pub id: UiId,
    pub label: &'static str,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StatusView {
    pub kind: StatusKind,
    pub text: String,
    pub dismiss_id: UiId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormView {
    pub scope: FormScope,
    pub title: &'static str,
    pub description: String,
    pub fields: Vec<FieldView>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeaderView {
    pub title: &'static str,
    pub description: String,
    pub back_id: UiId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ViewModel {
    pub tab: EventType,
    pub route: Route,
    pub status: Option<StatusView>,
    pub segments: Vec<SegmentView>,
    pub header: Option<HeaderView>,
    pub cards: Vec<CardBlock>,
    pub list: Option<EventListView>,
    pub form: Option<FormView>,
    pub primary: Option<ActionView>,
    pub secondary: Option<ActionView>,
    pub busy: bool,
}

/// 概览区块：说明卡（无跳转）或带箭头的可点击卡（进入子页）
#[derive(Debug, Clone, PartialEq)]
pub enum CardBlock {
    Note { title: &'static str, description: String, symbol: &'static str },
    Actionable { title: String, description: String, symbol: &'static str, event: String },
}

impl ViewModel {
    /// 结构 + 文案 + 状态的联合指纹；相等即界面无需重建（E-17 去重）
    pub fn fingerprint(&self) -> u64 {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let mut feed = |text: &str| {
            for byte in text.as_bytes() {
                hash ^= *byte as u64;
                hash = hash.wrapping_mul(0x1000_0000_01b3);
            }
            hash ^= 0xff;
            hash = hash.wrapping_mul(0x1000_0000_01b3);
        };
        feed(&format!("tab:{:?}|route:{:?}|busy:{}", self.tab, self.route, self.busy));
        if let Some(status) = &self.status {
            feed(&format!("status:{:?}:{}", status.kind, status.text));
        }
        for segment in &self.segments {
            feed(&format!("segment:{}:{}", segment.id.as_str(), segment.active));
        }
        if let Some(header) = &self.header {
            feed(&format!("header:{}:{}", header.title, header.description));
        }
        for card in &self.cards {
            match card {
                CardBlock::Note { title, description, .. } => feed(&format!("note:{}:{}", title, description)),
                CardBlock::Actionable { title, description, event, .. } => {
                    feed(&format!("card:{}:{}:{}", title, description, event))
                }
            }
        }
        if let Some(list) = &self.list {
            feed(&format!(
                "list:{}:{}:{}/{}:{}",
                list.scope.as_str(),
                list.page,
                list.max_page,
                list.total,
                list.footnote
            ));
            for row in &list.rows {
                feed(&format!("row:{}:{}:{}:{}", row.id, row.label, row.detail, row.selected));
            }
        }
        if let Some(form) = &self.form {
            feed(&format!("form:{}:{}", form.title, form.description));
            for field in &form.fields {
                match field {
                    FieldView::Text { id, value, .. } => feed(&format!("text:{}:{}", id.as_str(), value)),
                    FieldView::Select { id, value, options, .. } => feed(&format!("select:{}:{}:{:?}", id.as_str(), value, options)),
                    FieldView::Switch { on_id, off_id, on, .. } => {
                        feed(&format!("switch:{}{}:{}", on_id.as_str(), off_id.as_str(), on))
                    }
                }
            }
        }
        for action in [self.primary.as_ref(), self.secondary.as_ref()].into_iter().flatten() {
            feed(&format!("action:{}:{}:{}", action.id.as_str(), action.label, action.enabled));
        }
        hash
    }

    /// 该视图模型绑定了哪些事件标识（E-16 反向覆盖核对）
    pub fn bound_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = Vec::new();
        if let Some(status) = &self.status {
            ids.push(status.dismiss_id.as_str());
        }
        ids.extend(self.segments.iter().map(|segment| segment.id.as_str()));
        if let Some(header) = &self.header {
            ids.push(header.back_id.as_str());
        }
        for card in &self.cards {
            if let CardBlock::Actionable { event, .. } = card {
                ids.push(event.clone());
            }
        }
        if let Some(list) = &self.list {
            ids.extend(list.rows.iter().map(|row| row.id.clone()));
            ids.push(list.prev_id.as_str());
            ids.push(list.next_id.as_str());
        }
        if let Some(form) = &self.form {
            for field in &form.fields {
                match field {
                    FieldView::Text { id, .. } | FieldView::Select { id, .. } => ids.push(id.as_str()),
                    FieldView::Switch { on_id, off_id, .. } => {
                        ids.push(on_id.as_str());
                        ids.push(off_id.as_str());
                    }
                }
            }
        }
        for action in [self.primary.as_ref(), self.secondary.as_ref()].into_iter().flatten() {
            ids.push(action.id.as_str());
        }
        ids
    }
}

fn form_fields(scope: FormScope, state: &UiState) -> Vec<FieldView> {
    let Some(draft) = state.draft(scope) else {
        return Vec::new();
    };
    let mut fields = vec![
        FieldView::Text {
            id: UiId::Field(scope, FieldRole::Name),
            value: draft.name.clone(),
            label: FieldRole::Name.label(),
            hint: FieldRole::Name.hint(),
        },
        FieldView::Text {
            id: UiId::Field(scope, FieldRole::Date),
            value: draft.time.clone(),
            label: FieldRole::Date.label(),
            hint: FieldRole::Date.hint(),
        },
        FieldView::Switch {
            on_id: UiId::SwitchOn(scope, FieldRole::OnHome),
            off_id: UiId::SwitchOff(scope, FieldRole::OnHome),
            on: draft.on_index,
            label: FieldRole::OnHome.label(),
            hint: FieldRole::OnHome.hint(),
        },
        FieldView::Switch {
            on_id: UiId::SwitchOn(scope, FieldRole::CountStartDay),
            off_id: UiId::SwitchOff(scope, FieldRole::CountStartDay),
            on: draft.if_staring_day,
            label: FieldRole::CountStartDay.label(),
            hint: FieldRole::CountStartDay.hint(),
        },
        FieldView::Switch {
            on_id: UiId::SwitchOn(scope, FieldRole::Repeat),
            off_id: UiId::SwitchOff(scope, FieldRole::Repeat),
            on: draft.extensions.get("repeat").and_then(|v| v.as_str()) == Some("yearly"),
            label: FieldRole::Repeat.label(), hint: FieldRole::Repeat.hint(),
        },
        FieldView::Select {
            id: UiId::Field(scope, FieldRole::Unit),
            value: draft.extensions.get("displayUnit").and_then(|v| v.as_str()).unwrap_or("days").to_string(),
            options: vec![("days", "天"), ("weeks", "周"), ("months", "月"), ("years", "年")],
            label: FieldRole::Unit.label(), hint: FieldRole::Unit.hint(),
        },
    ];
    for (role, key) in [(FieldRole::Pinned, "pinned"), (FieldRole::Archived, "archived")] {
        fields.push(FieldView::Switch { on_id: UiId::SwitchOn(scope, role), off_id: UiId::SwitchOff(scope, role),
            on: draft.extensions.get(key).and_then(|v| v.as_bool()).unwrap_or(false), label: role.label(), hint: role.hint() });
    }
    fields.push(FieldView::Select { id: UiId::Field(scope, FieldRole::Category),
        value: draft.extensions.get("category").and_then(|v| v.as_str()).unwrap_or("uncategorized").into(),
        options: vec![("uncategorized", "未分类"), ("birthday", "生日"), ("study", "学习"), ("life", "生活"), ("anniversary", "纪念日")],
        label: FieldRole::Category.label(), hint: FieldRole::Category.hint() });
    fields
}

fn event_list(scope: FormScope, state: &UiState) -> EventListView {
    let max_page = state.max_page();
    // 列表缩短后页码可能越界，展示时统一收敛到有效范围
    let page = state.event_page.min(max_page.saturating_sub(1));
    let rows: Vec<EventRowView> = state
        .page_events()
        .into_iter()
        .map(|(index, event)| {
            let key = event.selection_key(index);
            let mut badges = vec![event.time.clone()];
            if event.extensions.get("pinned").and_then(|v| v.as_bool()) == Some(true) { badges.push("置顶".into()); }
            if event.extensions.get("archived").and_then(|v| v.as_bool()) == Some(true) { badges.push("已归档".into()); }
            badges.push(category_label(event.extensions.get("category").and_then(|v| v.as_str()).unwrap_or("uncategorized")).into());
            if event.extensions.get("repeat").and_then(|v| v.as_str()) == Some("yearly") {
                if let Some((date, Some(years))) = crate::ui::validation::occurrence(&event.time, "yearly", &state.today) {
                    badges.push(format!("下次 {} · 第{}周年", date, years));
                }
            }
            if event.on_index {
                badges.push("首页".to_string());
            }
            if !event.id.is_empty() && state.primary_id.as_deref() == Some(event.id.as_str()) {
                badges.push("表盘".to_string());
            }
            if event.if_staring_day {
                badges.push("含首日".to_string());
            }
            if event.id.is_empty() {
                badges.push("无ID".to_string());
            }
            EventRowView {
                id: pick_id(scope, &key),
                label: event.name.clone(),
                detail: badges.join(" · "),
                selected: state.selected_event_key.as_deref() == Some(key.as_str()),
                navigates: scope != FormScope::Delete,
            }
        })
        .collect();
    let total = state.filtered_events().len();
    let max_page = state.max_page();
    EventListView {
        scope,
        title: match scope {
            FormScope::Delete => "选择要删除的事件",
            _ => "腕端事件",
        },
        description: match scope {
            FormScope::Delete => "点击选中后按删除，删除前会再次确认名称与日期",
            _ => "点击事件进入编辑子页，未获取数据时先同步腕端",
        },
        rows,
        page,
        max_page,
        total,
        prev_id: UiId::PagePrev(scope),
        next_id: UiId::PageNext(scope),
        footnote: list_footnote(state, scope),
    }
}

    fn list_footnote(state: &UiState, scope: FormScope) -> String {
    if state.all_events.is_empty() {
        return if state.has_fetched_events {
            "腕端当前没有事件".to_string()
        } else {
            "尚未获取腕端数据".to_string()
        };
    }
    let page = state.event_page.min(state.max_page().saturating_sub(1));
    let base = format!(
        "共 {} 个事件 · 第 {} / {} 页",
        state.filtered_events().len(),
        page + 1,
        state.max_page()
    );
    if scope == FormScope::Delete {
        match state.selected_event_key.as_deref().and_then(|key| state.index_of_key(key)) {
            Some(index) => match state.all_events.get(index) {
                Some(event) => format!("{} · 已选「{} · {}」", base, event.name, event.time),
                None => base,
            },
            None => format!("{} · 尚未选择", base),
        }
    } else {
        base
    }
}

fn segments(state: &UiState) -> Vec<SegmentView> {
    EventType::ALL
        .into_iter()
        .map(|tab| SegmentView {
            id: UiId::Segment(tab),
            label: tab.label(),
            active: state.current_tab == tab,
        })
        .collect()
}

pub fn build(state: &UiState) -> ViewModel {
    let status = state.result_status.as_ref().or(state.status.as_ref()).map(|message| StatusView {
        kind: message.kind,
        text: message.text.clone(),
        dismiss_id: UiId::StatusDismiss,
    });
    let segments = segments(state);
    let busy = state.busy();
    let mut model = ViewModel {
        tab: state.current_tab,
        route: state.route,
        status,
        segments,
        header: None,
        cards: Vec::new(),
        list: None,
        form: None,
        primary: None,
        secondary: None,
        busy,
    };

    match (state.current_tab, state.route) {
        (EventType::AddEvent, _) => {
            model.cards.push(CardBlock::Note {
                title: "新建事件",
                description: "填写名称与目标日期，设置显示选项后发送到腕端；表盘事件请在修改页单独指定".to_string(),
                symbol: "plus",
            });
            model.form = Some(FormView {
                scope: FormScope::Add,
                title: "事件信息",
                description: reserved_note(),
                fields: form_fields(FormScope::Add, state),
            });
            model.primary = Some(ActionView {
                id: UiId::Submit(FormScope::Add),
                label: "发送到腕端".to_string(),
                kind: ButtonKind::Primary,
                enabled: !busy,
            });
        }
        (EventType::ModifyEvent, Route::Overview) => {
            model.cards.push(CardBlock::Note { title:"JSON 备份与恢复", symbol:"info",
                description:"先同步完整腕端列表，再导出备份或选择 JSON 预览；恢复前确认合并/替换与同 ID 冲突规则。".into() });
            if !busy {
                for (id, title) in [(UiId::ExportBackup, "导出完整 JSON 备份"), (UiId::ImportBackup, "选择 JSON 并预览恢复")] {
                    model.cards.push(CardBlock::Actionable {title:title.into(), description:"使用完整列表，包含归档事件与未知扩展".into(), symbol:"info", event:id.as_str()});
                }
            }
            model.cards.push(CardBlock::Note {
                title: "修改流程",
                description: "先同步腕端事件，再点事件卡片进入编辑子页；只提交改动的字段".to_string(),
                symbol: "edit",
            });
            model.list = Some(event_list(FormScope::Modify, state));
            model.primary = Some(ActionView {
                id: UiId::Fetch(FormScope::Modify),
                label: if state.has_fetched_events { "重新同步腕端" } else { "同步腕端数据" }.to_string(),
                kind: ButtonKind::AccentQuiet,
                enabled: !state.sending(),
            });
            model.secondary = state.selected_event_key.as_ref().map(|_| ActionView {
                id: UiId::Submit(FormScope::Modify),
                label: "同步修改".to_string(),
                kind: ButtonKind::Primary,
                enabled: !busy,
            });
        }
        (EventType::ModifyEvent, Route::Editor(scope)) => {
            let current = state.selected_event_key.as_deref().and_then(|key| state.index_of_key(key))
                .and_then(|index| state.all_events.get(index));
            let is_primary = current.is_some_and(|event| !event.id.is_empty() && state.primary_id.as_deref() == Some(event.id.as_str()));
            model.cards.push(CardBlock::Note { title: "表盘主事件", symbol: "watch",
                description: if is_primary { "当前事件是表盘主事件。取消后清空表盘，不自动指定其他事件。" }
                else { "当前事件不是表盘主事件。设置会替换原主事件，与首页展示、置顶无关。" }.into() });
            if !busy && current.is_some_and(|event| !event.id.is_empty()
                && (is_primary || event.extensions.get("archived").and_then(|v| v.as_bool()) != Some(true))) {
                model.cards.push(CardBlock::Actionable {
                    title: if is_primary { "取消表盘主事件" } else { "设为表盘主事件" }.into(),
                    description: "仅操作已保存事件，不提交编辑草稿；以腕端最终回执确认".into(), symbol: "watch",
                    event: if is_primary { UiId::CancelPrimary } else { UiId::SetPrimary }.as_str(),
                });
            }
            let label = selected_label(state);
            model.header = Some(HeaderView {
                title: "修改事件",
                description: label,
                back_id: UiId::BackOverview,
            });
            model.form = Some(FormView {
                scope,
                title: "事件信息",
                description: "未改动的字段保持腕端原值".to_string(),
                fields: form_fields(scope, state),
            });
            model.primary = Some(ActionView {
                id: UiId::Submit(FormScope::Modify),
                label: "同步修改".to_string(),
                kind: ButtonKind::Primary,
                enabled: !busy,
            });
        }
        (EventType::DeleteEvent, _) => {
            model.cards.push(CardBlock::Note {
                title: "删除会同步影响表盘",
                description: "腕端事件文件与表盘文件会一并更新，删除前会弹出确认框".to_string(),
                symbol: "trash",
            });
            model.list = Some(event_list(FormScope::Delete, state));
            model.primary = Some(ActionView {
                id: UiId::Submit(FormScope::Delete),
                label: "删除所选事件".to_string(),
                kind: ButtonKind::Danger,
                enabled: !busy && state.selected_event_key.is_some(),
            });
            model.secondary = Some(ActionView {
                id: UiId::Fetch(FormScope::Delete),
                label: if state.has_fetched_events { "重新同步腕端" } else { "同步腕端数据" }.to_string(),
                kind: ButtonKind::AccentQuiet,
                enabled: !state.sending(),
            });
        }
        (EventType::AddBackground, _) => {
            model.cards.push(CardBlock::Note {
                title: "背景图片",
                description: "支持 PNG / JPG / JPEG，压缩到最长边 450px 且不超过 100KB；新图发送成功前不会替换腕端当前背景".to_string(),
                symbol: "image",
            });
            model.primary = Some(ActionView {
                id: UiId::PickBackground,
                label: "选择背景图片".to_string(),
                kind: ButtonKind::Secondary,
                enabled: !busy,
            });
        }
    }
    if state.result_request.is_some() || state.legacy_request.is_some() {
        model.cards.push(CardBlock::Note {
            title: "原请求结果核实",
            description: "结果未知时先同步完整列表并核对事件。确认后可结束等待；这不代表腕端确认成功，也不会重发操作。".to_string(),
            symbol: "info",
        });
        model.secondary = Some(if state.can_finish_unknown() {
            ActionView { id: UiId::FinishUnknown, label: "已核实，结束等待".to_string(),
                kind: ButtonKind::AccentQuiet, enabled: true }
        } else {
            ActionView { id: UiId::Fetch(FormScope::Modify), label: "查询原结果并同步列表".to_string(),
                kind: ButtonKind::AccentQuiet, enabled: !state.sending() }
        });
    }
    if model.list.is_some() {
        let sort_label = match state.list_sort.as_str() { "near" => "临近日期", "manual" => "手动顺序", _ => "创建顺序" };
        for (id, title) in [(UiId::SortMode, format!("排序：{}（同步腕端）", sort_label)),
            (UiId::FilterCategory, format!("分类：{}", category_label(&state.list_category))),
            (UiId::FilterArchive, if state.list_archived { "已归档" } else { "正常事件" }.into())] {
            model.cards.push(CardBlock::Actionable { title, description: "点击切换".into(), symbol: "calendar", event: id.as_str() });
        }
    }
    if state.current_tab == EventType::ModifyEvent && state.route == Route::Overview {
        if let Some(preview) = &state.backup_preview {
            let incoming = preview.backup["data"]["events"].as_array().unwrap().len();
            model.cards.push(CardBlock::Note {title:"恢复预览", symbol:"info", description:format!(
                "备份 {} 条；合并后 {} 条。\n同 ID 冲突 {} 条：{}\n同名不同 ID {} 条（分别保留）：{}\n合并保留当前排序和主事件；替换使用备份排序和主事件。",
                incoming, preview.merged_count, preview.conflicts.len(), preview.conflicts.join("；"), preview.same_names.len(), preview.same_names.join("；"))});
            if !busy {
                for (id, title, description) in [
                    (UiId::BackupMode, if preview.replace {"模式：替换全部"} else {"模式：合并"}, "点击切换；替换会移除备份外的事件"),
                    (UiId::BackupConflict, if preview.overwrite {"同 ID：使用备份"} else {"同 ID：保留现有"}, "仅合并时生效；同名不同 ID 不覆盖"),
                    (UiId::CancelBackup, "取消恢复预览", "不写入腕端")
                ] { model.cards.push(CardBlock::Actionable {title:title.into(), description:description.into(), symbol:"info", event:id.as_str()}); }
            }
            model.primary = Some(ActionView {id:UiId::RestoreBackup, label:"确认按预览恢复".into(), kind:ButtonKind::Primary, enabled:!busy});
            if state.result_request.is_none() && state.legacy_request.is_none() {
                model.secondary = Some(ActionView {id:UiId::Fetch(FormScope::Modify), label:"重新同步腕端".into(), kind:ButtonKind::AccentQuiet, enabled:!state.sending() && !state.local_file_busy});
            }
        }
    }
    if matches!(state.route, Route::Editor(FormScope::Modify)) && !busy {
        for (id, title) in [(UiId::MoveUp, "前移"), (UiId::MoveDown, "后移")] {
            model.cards.push(CardBlock::Actionable { title: title.into(), description: "移动已保存事件，同置顶组/归档状态；草稿先同步修改".into(), symbol: "edit", event: id.as_str() });
        }
    }
    model
}

fn selected_label(state: &UiState) -> String {
    match state.selected_event_key.as_deref().and_then(|key| state.index_of_key(key)) {
        Some(index) => match state.all_events.get(index) {
            Some(event) => format!("{} · {}", event.name, event.time),
            None => "未选择事件".to_string(),
        },
        None => "未选择事件".to_string(),
    }
}

/// 决定是否需要重建界面：指纹未变返回 None，宿主据此跳过整树重画（E-17）
pub fn should_render(state: &mut UiState, force: bool) -> Option<ViewModel> {
    let view = build(state);
    let fingerprint = view.fingerprint();
    if !force && state.rendered_fingerprint == Some(fingerprint) {
        return None;
    }
    state.rendered_fingerprint = Some(fingerprint);
    Some(view)
}

/// 预留字段说明文案（F-01 / F-02 / F-03 落地时替换成真实控件）
pub fn reserved_note() -> String {
    "置顶、分类与归档需腕端确认对应能力；表盘主事件请在修改页单独指定".into()
}

fn category_label(key: &str) -> &'static str {
    match key { "birthday" => "生日", "study" => "学习", "life" => "生活", "anniversary" => "纪念日", "all" => "全部", _ => "未分类" }
}

#[cfg(test)]
mod select_tests {
    use super::*;
    use crate::ui::dispatch::{resolve, apply_action, UiEventKind};

    #[test]
    fn primary_entry_tracks_confirmed_id_and_hides_while_busy_or_archived() {
        let mut state = UiState::new("2026-10-08".into());
        state.current_tab = EventType::ModifyEvent;
        state.all_events.push(crate::ui::state::EventData {id:"e1".into(), ..Default::default()});
        state.open_editor("id:e1");
        let has = |state: &UiState, id: UiId| build(state).bound_ids().contains(&id.as_str());
        assert!(has(&state, UiId::SetPrimary));
        state.primary_id = Some("e1".into());
        assert!(has(&state, UiId::CancelPrimary));
        state.awaiting_confirmation = true;
        assert!(!has(&state, UiId::CancelPrimary));
        state.awaiting_confirmation = false;
        state.primary_id = Some("".into());
        state.all_events[0].extensions.insert("archived".into(), serde_json::json!(true));
        assert!(!has(&state, UiId::SetPrimary));
        assert!(state.primary_payload(false).is_err());
    }

    #[test]
    fn primary_receipt_requires_final_metadata_and_preserves_draft_on_failure() {
        for (ok, watchface_ok, code) in [(true, false, "WATCHFACE_FAIL"), (false, false, "WRITE_FAIL")] {
            let mut state = UiState::new("2026-10-08".into());
            state.target_device = Some("watch".into());
            state.snapshot_device = Some("watch".into());
            state.primary_id = Some("old".into());
            state.all_events.push(crate::ui::state::EventData {id:"e1".into(), ..Default::default()});
            state.open_editor("id:e1");
            state.modify_event_data.name = "草稿".into();
            let token = state.reserve_send(state.primary_payload(false).unwrap(), crate::ui::state::SendPurpose::Mutation).unwrap();
            state.attach_send_device(token, "watch".into());
            let pending = state.claim_send(token).unwrap();
            state.begin_result_wait(&pending);
            let mut result = serde_json::json!({"type":"mutationResult", "requestId":pending.request_id,
                "sessionId":pending.session_id,"deviceId":"watch", "ok":ok, "code":code});
            if ok {
                assert!(!state.apply_request_message(&result));
                assert!(state.busy());
                result["revision"] = serde_json::json!(11);
                result["primaryId"] = serde_json::json!("e1");
                result["primary"] = serde_json::json!({"ok":true});
                result["watchface"] = serde_json::json!({"ok":watchface_ok});
            }
            assert!(state.apply_request_message(&result));
            assert!(!state.busy());
            assert_eq!(state.primary_id.as_deref(), Some(if ok {"e1"} else {"old"}));
            assert_eq!(state.modify_event_data.name, "草稿");
            assert_eq!(state.result_status.as_ref().unwrap().kind, StatusKind::Error);
        }
    }

    #[test]
    fn 单位分类中文选择回填协议值且重绘保留选中项() {
        let mut state = UiState::new("2026-10-08".into());
        for (role, selected, label) in [(FieldRole::Unit, "weeks", "周"), (FieldRole::Category, "birthday", "生日")] {
            let id = UiId::Field(FormScope::Add, role).as_str();
            let action = resolve(&id, UiEventKind::Change, &format!(r#"{{"value":"{}"}}"#, selected)).unwrap();
            apply_action(&mut state, action);
            let fields = form_fields(FormScope::Add, &state);
            let field = fields.iter().find(|f| matches!(f, FieldView::Select {id, ..} if *id == UiId::Field(FormScope::Add, role))).unwrap();
            match field {
                FieldView::Select {value, options, ..} => {
                    assert_eq!(value, selected);
                    assert!(options.contains(&(selected, label)));
                }
                _ => unreachable!(),
            }
        }
        let payload = crate::ui::protocol::add_event(&state.event_data);
        let value: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(value["displayUnit"], "weeks");
        assert_eq!(value["category"], "birthday");
        assert!(!build(&state).cards.iter().any(|c| matches!(c, CardBlock::Actionable {event, ..} if event == "action.add.category")));
    }
}

/// 分页文案
pub fn page_hint(page: usize, max_page: usize) -> String {
    format!("{} / {}", page + 1, max_page)
}

pub fn empty_list_hint() -> &'static str {
    "尚未获取腕端数据"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::dispatch::{Action, UiEventKind};
    use crate::ui::state::{EventData, StatusMessage};

    fn state_with_events() -> UiState {
        let mut state = UiState::new("2026-10-06".to_string());
        state.has_fetched_events = true;
        state.all_events = vec![
            EventData {
                id: "e1".into(),
                name: "同名".into(),
                time: "2026-01-01".into(),
                on_index: true,
                ..Default::default()
            },
            EventData {
                id: "e2".into(),
                name: "同名".into(),
                time: "2026-02-02".into(),
                if_staring_day: true,
                ..Default::default()
            },
        ];
        state
    }

    fn rows_of(view: &ViewModel) -> Vec<EventRowView> {
        view.list.as_ref().map(|list| list.rows.clone()).unwrap_or_default()
    }

    #[test]
    fn 四个分段都能构建且当前段高亮() {
        for tab in EventType::ALL {
            let mut state = UiState::new("2026-10-06".to_string());
            state.current_tab = tab;
            let view = build(&state);
            assert_eq!(view.segments.len(), 4);
            assert_eq!(view.segments.iter().filter(|item| item.active).count(), 1);
            assert_eq!(view.segments.iter().find(|item| item.active).unwrap().label, tab.label());
            assert!(view.primary.is_some(), "{} 页必须有单一主操作", tab.label());
        }
    }

    #[test]
    fn 修改概览是列表页_编辑是带返回的子页() {
        let mut state = state_with_events();
        state.current_tab = EventType::ModifyEvent;
        let overview = build(&state);
        assert!(overview.header.is_none(), "概览页不显示返回按钮");
        assert!(overview.list.is_some());
        assert_eq!(overview.primary.as_ref().unwrap().label, "重新同步腕端");
        assert!(overview.secondary.is_none(), "未选择事件时不显示同步修改");

        state.open_editor("id:e1");
        let editor = build(&state);
        let header = editor.header.as_ref().unwrap();
        assert_eq!(header.title, "修改事件");
        assert!(header.description.contains("同名 · 2026-01-01"));
        assert!(editor.list.is_none(), "子页不重复渲染列表");
        assert!(editor.form.is_some());
        assert_eq!(editor.primary.as_ref().unwrap().label, "同步修改");
        assert!(editor.bound_ids().contains(&"nav.back".to_string()));
    }

    #[test]
    fn 事件行带可读标签且修改页行可进入子页() {
        let mut state = state_with_events();
        state.current_tab = EventType::ModifyEvent;
        let rows = rows_of(&build(&state));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, "pick.modify.id:e1");
        assert_eq!(rows[0].label, "同名");
        assert_eq!(rows[0].detail, "2026-01-01 · 未分类 · 首页");
        assert!(rows[0].navigates);
        let action = crate::ui::dispatch::resolve(&rows[0].id, UiEventKind::Click, "").expect("行可点击");
        assert_eq!(action, Action::PickEvent { scope: FormScope::Modify, key: "id:e1".into() });

        let mut applied = state_with_events();
        applied.current_tab = EventType::ModifyEvent;
        crate::ui::dispatch::apply_action(&mut applied, action);
        assert_eq!(applied.route, Route::Editor(FormScope::Modify), "点击行应进入编辑子页");
    }

    #[test]
    fn 删除页行原地选中并进入脚注() {
        let mut state = state_with_events();
        state.current_tab = EventType::DeleteEvent;
        assert_eq!(build(&state).primary.as_ref().unwrap().enabled, false, "未选择时删除按钮禁用");
        let row = rows_of(&build(&state))[1].clone();
        assert!(!row.navigates, "删除页不进入子页");
        let action = crate::ui::dispatch::resolve(&row.id, UiEventKind::Click, "").unwrap();
        crate::ui::dispatch::apply_action(&mut state, action);
        let view = build(&state);
        assert_eq!(view.route, Route::Overview);
        assert!(view.primary.as_ref().unwrap().enabled);
        assert!(view.list.as_ref().unwrap().footnote.contains("已选「同名 · 2026-02-02」"));
        assert_eq!(view.list.as_ref().unwrap().rows[1].detail, "2026-02-02 · 未分类 · 含首日");
    }

    #[test]
    fn 在途时主操作禁用() {
        let mut state = state_with_events();
        state.current_tab = EventType::AddEvent;
        state.pending_send = Some(crate::ui::state::PendingSend {
            token: 1,
            device_addr: "x".into(),
            payload: "{}".into(),
            attempts: 0,
            in_progress: true,
            purpose: crate::ui::state::SendPurpose::Mutation,
            base_revision: None,
            request_id: String::new(),
            session_id: String::new(),
            device_id: String::new(),
        });
        let view = build(&state);
        assert!(view.busy);
        assert!(!view.primary.as_ref().unwrap().enabled, "在途期间主操作必须禁用");
    }

    #[test]
    fn 在途时重复派发被忽略() {
        let mut state = UiState::new("2026-10-06".to_string());
        state.pending_send = Some(crate::ui::state::PendingSend {
            token: 1,
            device_addr: "x".into(),
            payload: "{}".into(),
            attempts: 0,
            in_progress: true,
            purpose: crate::ui::state::SendPurpose::Mutation,
            base_revision: None,
            request_id: String::new(),
            session_id: String::new(),
            device_id: String::new(),
        });
        let change = crate::ui::dispatch::apply_action(&mut state, Action::SubmitAdd);
        assert!(!change.rebuild);
        assert!(change.status.is_none(), "锁定期间静默忽略，不弹提示");
    }

    #[test]
    fn 状态面板常驻且可关闭() {
        let mut state = UiState::new("2026-10-06".to_string());
        assert!(build(&state).status.is_none());
        state.status = Some(StatusMessage::success("已发送，等待腕端确认"));
        let view = build(&state);
        assert_eq!(view.status.as_ref().unwrap().kind, StatusKind::Success);
        assert!(view.bound_ids().contains(&"status.dismiss".to_string()));
        crate::ui::dispatch::apply_action(&mut state, Action::DismissStatus);
        assert!(build(&state).status.is_none());
    }

    #[test]
    fn 绑定标识都能反解且无未知标识() {
        let mut state = state_with_events();
        for tab in EventType::ALL {
            state.current_tab = tab;
            for route in [Route::Overview, Route::Editor(FormScope::Modify)] {
                state.route = route;
                for id in build(&state).bound_ids() {
                    assert!(
                        UiId::parse(&id).is_some() || crate::ui::dispatch::parse_pick(&id).is_some(),
                        "{} / {:?} 出现未知标识 {}",
                        tab.label(),
                        route,
                        id
                    );
                }
            }
        }
    }

    #[test]
    fn 指纹未变时不重建_关键状态变化才重画() {
        let mut state = state_with_events();
        assert!(should_render(&mut state, false).is_some());
        assert!(should_render(&mut state, false).is_none());
        assert!(should_render(&mut state, true).is_some(), "强制渲染忽略指纹");

        state.current_tab = EventType::ModifyEvent;
        assert!(should_render(&mut state, false).is_some());
        state.event_data.name = "草稿".into();
        assert!(should_render(&mut state, false).is_none(), "非当前页草稿不触发重建");

        state.open_editor("id:e1");
        assert!(should_render(&mut state, false).is_some(), "进入子页必须重画");

        state.status = Some(StatusMessage::info("正在发送"));
        assert!(should_render(&mut state, false).is_some(), "状态提示必须重画");

        state.pending_send = Some(crate::ui::state::PendingSend {
            token: 1,
            device_addr: "x".into(),
            payload: "{}".into(),
            attempts: 0,
            in_progress: true,
            purpose: crate::ui::state::SendPurpose::Mutation,
            base_revision: None,
            request_id: String::new(),
            session_id: String::new(),
            device_id: String::new(),
        });
        assert!(should_render(&mut state, false).is_some(), "在途锁定必须重画");
    }

    #[test]
    fn 添加页字段顺序与说明固定() {
        let state = UiState::new("2026-10-06".to_string());
        let fields = form_fields(FormScope::Add, &state);
        assert_eq!(fields.len(), 9);
        assert!(matches!(fields[0], FieldView::Text { .. }));
        assert!(matches!(fields[1], FieldView::Text { .. }));
        assert!(matches!(fields[2], FieldView::Switch { on: true, .. }));
        assert!(matches!(fields[3], FieldView::Switch { on: false, .. }));
        assert_eq!(build(&state).form.as_ref().unwrap().description, reserved_note());
    }

    #[test]
    fn 两个开关选项无论当前选中态都保留点击动作() {
        for on in [true, false] {
            let state = UiState::new("2026-10-06".to_string());
            let fields = form_fields(FormScope::Add, &state);
            let switch = fields.iter().find_map(|field| match field {
                FieldView::Switch { on_id, off_id, .. } if *on_id == UiId::SwitchOn(FormScope::Add, FieldRole::OnHome) => {
                    Some((*on_id, *off_id))
                }
                _ => None,
            }).unwrap();
            for id in [switch.0, switch.1] {
                assert!(
                    crate::ui::dispatch::resolve(&id.as_str(), crate::ui::dispatch::UiEventKind::Click, "").is_some(),
                    "选中态 {} 下两个开关选项都必须可点击",
                    on
                );
            }
        }
    }

    #[test]
    fn 未获取数据时列表有引导文案() {
        let mut state = UiState::new("2026-10-06".to_string());
        state.current_tab = EventType::ModifyEvent;
        let view = build(&state);
        let list = view.list.as_ref().unwrap();
        assert!(list.rows.is_empty());
        assert!(list.footnote.contains("尚未获取"));
        assert_eq!(page_hint(0, 1), "1 / 1");
        assert_eq!(empty_list_hint(), "尚未获取腕端数据");
    }

    #[test]
    fn 分页在列表里体现且越界收敛() {
        let mut state = UiState::new("2026-10-06".to_string());
        state.has_fetched_events = true;
        for index in 0..10 {
            state.all_events.push(EventData {
                id: format!("e{}", index),
                name: format!("事件{}", index),
                time: "2026-01-01".into(),
                ..Default::default()
            });
        }
        state.current_tab = EventType::ModifyEvent;
        assert_eq!(rows_of(&build(&state)).len(), crate::ui::theme::EVENT_PAGE_SIZE);
        assert_eq!(build(&state).list.as_ref().unwrap().max_page, 2);
        state.event_page = 5;
        assert_eq!(build(&state).list.as_ref().unwrap().page, 1, "越界页码收敛到最后一页");
    }

    #[test]
    fn 每页恰好一个主操作() {
        for tab in EventType::ALL {
            let mut state = state_with_events();
            state.current_tab = tab;
            let view = build(&state);
            let primaries = usize::from(view.primary.is_some());
            assert_eq!(primaries, 1, "{} 页主操作数量异常", tab.label());
        }
    }
}
