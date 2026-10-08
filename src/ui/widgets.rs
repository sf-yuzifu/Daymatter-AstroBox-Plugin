//! ClaraLight 共享 UI helper（与腕上漫画插件同源）：列 / 行 / 文本 / 卡片 / 输入框 /
//! 按钮 / 圆形返回 / 子页头 / 分段控制器 / 图标 / 分页器。宿主层，仅 wasm32 编译。

use crate::astrobox::psys_host_v4::ui;
use crate::ui::theme::{self, ButtonKind};

pub fn column() -> ui::Element {
    ui::Element::new(ui::ElementType::Div, None)
        .flex()
        .flex_direction(ui::FlexDirection::Column)
        .width_full()
        .min_width(0)
        .gap(theme::GAP)
}

pub fn row() -> ui::Element {
    ui::Element::new(ui::ElementType::Div, None)
        .flex()
        .flex_direction(ui::FlexDirection::Row)
        .align_center()
        .width_full()
        .min_width(0)
        .gap(theme::GAP_SM)
}

pub fn text(value: &str, size: u32, color: &str) -> ui::Element {
    ui::Element::new(ui::ElementType::P, Some(value))
        .size(size)
        .text_color(color)
        .min_width(0)
        .prop("style", "overflow-wrap:anywhere; margin:0")
}

pub fn hint(value: &str) -> ui::Element {
    text(value, theme::font_hint(), theme::MUTED)
}

pub fn panel() -> ui::Element {
    column().bg(theme::PANEL).radius(theme::RADIUS).border(1, theme::OUTLINE).padding(theme::PAD)
}

pub fn input(value: &str, placeholder: &str, event: &str) -> ui::Element {
    ui::Element::new(ui::ElementType::Input, Some(value))
        .without_default_styles()
        .on(ui::Event::Change, event)
        .prop("placeholder", placeholder)
        .height(theme::INPUT_HEIGHT)
        .width_full()
        .min_width(0)
        .radius(12)
        .border(1, theme::OUTLINE)
        .bg(theme::CONTROL)
        .text_color(theme::TEXT)
        .size(theme::font_body())
        .padding_left(14)
        .padding_right(14)
}

pub fn button(label: &str, event: &str, kind: ButtonKind, enabled: bool) -> ui::Element {
    let (background, foreground, border_color) = theme::button_palette(kind);
    let mut element = ui::Element::new(ui::ElementType::Button, None)
        .without_default_styles()
        .radius(12)
        .min_height(theme::ROW_HEIGHT)
        .padding_left(16)
        .padding_right(16)
        .padding_top(8)
        .padding_bottom(8)
        .bg(background)
        .text_color(foreground)
        .flex()
        .align_center()
        .justify_center()
        .min_width(0)
        .prop("aria-label", label)
        .child(text(label, theme::font_label(), foreground));
    if border_color != "transparent" {
        element = element.border(1, border_color);
    }
    if enabled {
        element = element.on(ui::Event::Click, event);
    } else {
        element = element.disabled().opacity(0.35);
    }
    element
}

/// AstroBox / ClaraLight 圆形导航返回按钮（36x36 纯圆 + 矢量箭头）
pub fn back_button(event: &str) -> ui::Element {
    ui::Element::new(ui::ElementType::Button, None)
        .without_default_styles()
        .on(ui::Event::Click, event)
        .width(theme::BACK_SIZE)
        .height(theme::BACK_SIZE)
        .radius(theme::RADIUS_PILL)
        .bg(theme::CONTROL)
        .border(1, theme::OUTLINE)
        .flex()
        .align_center()
        .justify_center()
        .flex_shrink(0.0)
        .prop("aria-label", "返回")
        .child(icon("back", theme::TEXT, 18))
}

/// 子页头：圆形返回 + 大标题 + 辅助描述
pub fn subpage_header(title: &str, description: &str, back_event: &str) -> ui::Element {
    let top = row()
        .gap(theme::GAP)
        .child(back_button(back_event))
        .child(text(title, theme::font_title(), theme::TEXT).flex_grow(1.0));
    column().gap(6).child(top).child(hint(description).margin_left(theme::BACK_SIZE + 12))
}

/// ClaraLight 分段控制器：药丸底座 + 互斥高亮，未选中项才可点
pub fn segmented_control(items: &[(&str, &str, bool)]) -> ui::Element {
    let mut root = ui::Element::new(ui::ElementType::Div, None)
        .flex()
        .flex_direction(ui::FlexDirection::Row)
        .align_center()
        .width_full()
        .min_width(0)
        .bg(theme::SEGMENT_BASE)
        .radius(14)
        .border(1, theme::OUTLINE)
        .padding(theme::GAP_XS)
        .gap(theme::GAP_XS);
    for (label, event, active) in items {
        let (background, foreground) = theme::segment_palette(*active);
        let mut item = ui::Element::new(ui::ElementType::Button, None)
            .without_default_styles()
            .radius(theme::RADIUS_SM)
            .height(theme::SEGMENT_HEIGHT)
            .flex()
            .align_center()
            .justify_center()
            .flex_grow(1.0)
            .min_width(0)
            .bg(background)
            .text_color(foreground)
            .prop("aria-label", label)
            .child(text(label, theme::font_label(), foreground));
        if !*active {
            item = item.on(ui::Event::Click, event);
        }
        root = root.child(item);
    }
    root
}

pub fn icon(name: &str, color: &str, size: u32) -> ui::Element {
    let path = match name {
        "plus" => "<line x1='12' y1='5' x2='12' y2='19'/><line x1='5' y1='12' x2='19' y2='12'/>",
        "edit" => "<path d='M12 20h9'/><path d='M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4Z'/>",
        "trash" => "<polyline points='3 6 5 6 21 6'/><path d='M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2'/>",
        "image" => "<rect x='3' y='3' width='18' height='18' rx='3'/><path d='m3 16 5-5 4 4 3-3 6 6'/><circle cx='15' cy='8' r='1.5'/>",
        "calendar" => "<rect x='3' y='5' width='18' height='16' rx='3'/><line x1='3' y1='10' x2='21' y2='10'/><line x1='8' y1='3' x2='8' y2='7'/><line x1='16' y1='3' x2='16' y2='7'/>",
        "watch" => "<circle cx='12' cy='12' r='9'/><polyline points='12 7 12 12 15 14'/>",
        "back" => "<polyline points='15 18 9 12 15 6'/>",
        "chevron" => "<polyline points='9 18 15 12 9 6'/>",
        _ => "<circle cx='12' cy='12' r='9'/>",
    };
    let svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'>{path}</svg>"
    );
    ui::Element::new(ui::ElementType::Svg, Some(&svg))
        .width(size)
        .height(size)
        .flex_shrink(0.0)
        .text_color(color)
}

pub fn note_card(title: &str, description: &str, symbol: &str) -> ui::Element {
    let icon_box = ui::Element::new(ui::ElementType::Div, None)
        .width(theme::ICON_BOX)
        .height(theme::ICON_BOX)
        .radius(theme::RADIUS_SM)
        .bg(theme::CONTROL)
        .flex()
        .align_center()
        .justify_center()
        .flex_shrink(0.0)
        .child(icon(symbol, theme::SECONDARY, 20));
    let info = column()
        .gap(3)
        .flex_grow(1.0)
        .child(text(title, theme::font_card_title(), theme::TEXT))
        .child(hint(description));
    row()
        .bg(theme::PANEL)
        .radius(theme::RADIUS)
        .border(1, theme::OUTLINE)
        .padding(theme::PAD_SM)
        .gap(theme::GAP)
        .child(icon_box)
        .child(info)
}

/// 可点击概览卡：图标 + 标题 + 描述 + 右箭头
pub fn action_card(title: &str, description: &str, symbol: &str, event: &str) -> ui::Element {
    let mut card = note_card(title, description, symbol);
    card = card
        .on(ui::Event::Click, event)
        .prop("role", "button")
        .tab_index(0)
        .border(1, theme::OUTLINE_STRONG)
        .child(icon("chevron", theme::MUTED, 18));
    card
}

/// 事件行：名称 + 徽标 + 可选箭头 / 选中态
pub fn event_row(title: &str, detail: &str, event: &str, selected: bool, navigates: bool) -> ui::Element {
    let (outline, background) = theme::row_outline(selected);
    let mut content = row()
        .flex_grow(1.0)
        .min_width(0)
        .child(text(title, theme::font_body(), theme::TEXT));
    if !detail.is_empty() {
        content = content.child(
            ui::Element::new(ui::ElementType::P, Some(detail))
                .size(theme::font_hint())
                .text_color(if selected { theme::ACCENT } else { theme::MUTED })
                .flex_shrink(0.0)
                .prop("style", "overflow-wrap:anywhere; margin:0"),
        );
    }
    let mut element = row()
        .on(ui::Event::Click, event)
        .prop("role", "button")
        .tab_index(0)
        .radius(theme::RADIUS_SM)
        .bg(background)
        .border(1, outline)
        .padding(12)
        .min_width(0)
        .child(content);
    if navigates {
        element = element.child(icon("chevron", theme::MUTED, 16));
    }
    element
}

/// 分页器：上一页 / 页码 / 下一页，边界按钮禁用
pub fn pager(page: usize, max_page: usize, prev: &str, next: &str) -> ui::Element {
    row()
        .gap(theme::GAP_SM)
        .child(button("上一页", prev, ButtonKind::Secondary, page > 0).flex_grow(1.0))
        .child(
            text(&format!("{} / {}", page + 1, max_page), theme::font_hint(), theme::MUTED).flex_shrink(0.0),
        )
        .child(button("下一页", next, ButtonKind::Secondary, page + 1 < max_page).flex_grow(1.0))
}

/// 状态面板：常驻展示，可点击关闭
pub fn status_panel(kind: crate::ui::state::StatusKind, message: &str, dismiss: &str) -> ui::Element {
    let (background, foreground, border_color) = theme::status_palette(kind);
    let symbol = match kind {
        crate::ui::state::StatusKind::Success => "watch",
        crate::ui::state::StatusKind::Error => "trash",
        crate::ui::state::StatusKind::Info => "calendar",
    };
    row()
        .gap(theme::GAP_SM)
        .bg(background)
        .border(1, border_color)
        .radius(theme::RADIUS_SM)
        .padding(12)
        .on(ui::Event::Click, dismiss)
        .prop("role", "button")
        .prop("aria-label", "关闭提示")
        .child(icon(symbol, foreground, 18))
        .child(text(message, theme::font_label(), foreground).flex_grow(1.0))
}

/// 表单字段块：标签 + 控件 + 说明
pub fn field_block(label: &str, control: ui::Element, tip: &str) -> ui::Element {
    column()
        .gap(6)
        .child(text(label, theme::font_label(), theme::SECONDARY))
        .child(control)
        .child(hint(tip))
}

/// 开关：两枚药丸按钮互斥高亮
pub fn select(value: &str, options: &[(&str, &str)], event: &str) -> ui::Element {
    let mut element = ui::Element::new(ui::ElementType::Select, None)
        .prop("value", value)
        .on(ui::Event::Change, event)
        .width_full()
        .min_width(0)
        .min_height(theme::ROW_HEIGHT)
        .bg(theme::CONTROL)
        .text_color(theme::TEXT)
        .radius(12)
        .padding_left(16)
        .padding_right(16);
    for (key, label) in options {
        let mut option = ui::Element::new(ui::ElementType::Option, Some(label))
            .prop("value", key);
        if *key == value { option = option.prop("selected", "true"); }
        element = element.child(option);
    }
    element
}

/// 开关：两枚药丸按钮互斥高亮
pub fn switch_pair(on_label: &str, off_label: &str, on_event: &str, off_event: &str, on: bool) -> ui::Element {
    row()
        .gap(theme::GAP_SM)
        .child(switch_option(on_label, on_event, on).flex_grow(1.0))
        .child(switch_option(off_label, off_event, !on).flex_grow(1.0))
}

/// 选中态只影响外观，两项始终可点击以便切换。
fn switch_option(label: &str, event: &str, selected: bool) -> ui::Element {
    ui::Element::new(ui::ElementType::Button, None)
        .without_default_styles()
        .on(ui::Event::Click, event)
        .radius(12)
        .min_height(theme::ROW_HEIGHT)
        .padding_left(16)
        .padding_right(16)
        .padding_top(8)
        .padding_bottom(8)
        .bg(if selected { theme::ACCENT } else { theme::CONTROL })
        .text_color(if selected { theme::TEXT } else { theme::SECONDARY })
        .border(1, if selected { theme::ACCENT } else { theme::OUTLINE })
        .flex()
        .align_center()
        .justify_center()
        .min_width(0)
        .prop("aria-pressed", if selected { "true" } else { "false" })
        .child(text(
            label,
            theme::font_label(),
            if selected { theme::TEXT } else { theme::SECONDARY },
        ))
}
