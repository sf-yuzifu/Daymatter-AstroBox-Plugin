//! 宿主渲染层：把 ClaraLight 视图模型翻译成 AstroBox 元素树，并按指纹去重重建（E-17 / E-20）。
//!
//! 结构：状态面板 → 分段控制器 → 子页头（仅子页）→ 概览卡片 / 事件列表 / 表单 → 底部单一主操作。

use crate::astrobox::psys_host_v4::ui;
use crate::ui::state::write_state;
use crate::ui::theme;
use crate::ui::view::{
    should_render, ActionView, CardBlock, EventListView, FieldView, FormView, HeaderView, SegmentView, StatusView,
    ViewModel,
};
use crate::ui::widgets as w;

fn segments(items: &[SegmentView]) -> ui::Element {
    let borrowed: Vec<(String, String, bool)> = items
        .iter()
        .map(|segment| (segment.label.to_string(), segment.id.as_str(), segment.active))
        .collect();
    let refs: Vec<(&str, &str, bool)> = borrowed
        .iter()
        .map(|(label, event, active)| (label.as_str(), event.as_str(), *active))
        .collect();
    w::segmented_control(&refs)
}

fn status_panel(status: &StatusView) -> ui::Element {
    w::status_panel(status.kind, &status.text, &status.dismiss_id.as_str())
}

fn header(header: &HeaderView) -> ui::Element {
    w::subpage_header(header.title, &header.description, &header.back_id.as_str())
}

fn cards(blocks: &[CardBlock]) -> Vec<ui::Element> {
    blocks
        .iter()
        .map(|block| match block {
            CardBlock::Note { title, description, symbol } => w::note_card(title, description, symbol),
            CardBlock::Actionable { title, description, symbol, event } => {
                w::action_card(title, description, symbol, event)
            }
            CardBlock::Image {source, description, width, height} => w::panel()
                .child(w::row().justify_center().child(
                    ui::Element::new(ui::ElementType::Image, Some(source)).width(*width).height(*height)
                        .flex_shrink(0.0).prop("style", "object-fit:contain;background:#000")))
                .child(w::hint(description)),
        })
        .collect()
}

fn list(list: &EventListView) -> ui::Element {
    let mut element = w::panel().child(w::text(list.title, theme::font_card_title(), theme::TEXT));
    element = element.child(w::hint(list.description));
    for row in &list.rows {
        element = element.child(w::event_row(&row.label, &row.detail, &row.id, row.selected, row.navigates));
    }
    if list.max_page > 1 {
        element = element.child(w::pager(list.page, list.max_page, &list.prev_id.as_str(), &list.next_id.as_str()));
    }
    // 空列表的引导文案统一放在脚注里，不重复渲染
    element.child(w::hint(&list.footnote))
}

fn field(field: &FieldView) -> ui::Element {
    match field {
        FieldView::Text { id, value, label, hint } => {
            w::field_block(label, w::input(value, hint, &id.as_str()), hint)
        }
        FieldView::Select { id, value, options, label, hint } => {
            w::field_block(label, w::select(value, options, &id.as_str()), hint)
        }
        FieldView::Switch { on_id, off_id, on, label, hint } => w::field_block(
            label,
            w::switch_pair("是", "否", &on_id.as_str(), &off_id.as_str(), *on),
            hint,
        ),
    }
}

fn form(form: &FormView) -> ui::Element {
    let mut element = w::panel().child(w::text(form.title, theme::font_card_title(), theme::TEXT));
    element = element.child(w::hint(&form.description));
    for field in &form.fields {
        element = element.child(self::field(field));
    }
    element
}

fn action_bar(primary: &Option<ActionView>, secondary: &Option<ActionView>) -> ui::Element {
    let mut element = w::row().gap(theme::GAP_SM);
    if let Some(action) = secondary {
        element = element.child(w::button(&action.label, &action.id.as_str(), action.kind, action.enabled).flex_grow(1.0));
    }
    if let Some(action) = primary {
        let mut button = w::button(&action.label, &action.id.as_str(), action.kind, action.enabled);
        if secondary.is_none() {
            button = button.width_full();
        } else {
            button = button.flex_grow(1.0);
        }
        element = element.child(button);
    }
    element
}

pub fn build_tree(view: &ViewModel) -> ui::Element {
    let mut root = w::column().bg(theme::CANVAS).padding(theme::PAD).gap(theme::GAP);
    if let Some(status) = &view.status {
        root = root.child(status_panel(status));
    }
    root = root.child(segments(&view.segments));
    if let Some(header) = &view.header {
        root = root.child(self::header(header));
    }
    for card in cards(&view.cards) {
        root = root.child(card);
    }
    if let Some(list) = &view.list {
        root = root.child(self::list(list));
    }
    if let Some(form) = &view.form {
        root = root.child(self::form(form));
    }
    root.child(action_bar(&view.primary, &view.secondary))
}

pub struct RenderRequest {
    root_id: String,
    tree: ui::Element,
}

/// 持锁时只准备纯渲染请求；不调用宿主，避免锁重入与宿主回调死锁。
pub fn prepare_with(state: &mut crate::ui::state::UiState, force: bool) -> Option<RenderRequest> {
    let Some(view) = should_render(state, force) else {
        return None;
    };
    let Some(root_id) = state.root_element_id.clone() else {
        return None;
    };
    Some(RenderRequest { root_id, tree: build_tree(&view) })
}

/// 必须在状态锁释放后调用宿主渲染。
pub fn render_prepared(request: Option<RenderRequest>) {
    if let Some(request) = request {
        ui::render(&request.root_id, request.tree);
    }
}

/// 调用方未持锁时使用：内部取锁后渲染
pub fn refresh(force: bool) {
    let request = {
        let mut state = write_state();
        prepare_with(&mut state, force)
    };
    render_prepared(request);
}

/// 宿主请求渲染根元素：总是重画（首次渲染 / 宿主主动刷新）
pub fn render_main_ui(element_id: &str) {
    let request = {
        let mut state = write_state();
        state.root_element_id = Some(element_id.to_string());
        prepare_with(&mut state, true)
    };
    render_prepared(request);
}
