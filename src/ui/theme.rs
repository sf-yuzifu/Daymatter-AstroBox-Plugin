//! ClaraLight 视觉令牌与纯样式决策（与腕上漫画插件同源，E-16 / E-20 可在 host 上测试）。
//! 只放令牌与"给定语义 → 给定颜色"的纯函数；元素拼装在宿主层 `widgets.rs`。

use crate::ui::state::StatusKind;

pub const CANVAS: &str = "#191919";
pub const PANEL: &str = "#1E1E1F";
pub const CONTROL: &str = "rgba(255, 255, 255, 0.08)";
pub const CONTROL_ACTIVE: &str = "rgba(255, 255, 255, 0.14)";
pub const SEGMENT_BASE: &str = "rgba(255, 255, 255, 0.05)";
pub const OUTLINE: &str = "rgba(255, 255, 255, 0.08)";
pub const OUTLINE_STRONG: &str = "rgba(255, 255, 255, 0.16)";
pub const TEXT: &str = "#FFFFFF";
pub const SECONDARY: &str = "rgba(255, 255, 255, 0.75)";
pub const MUTED: &str = "rgba(255, 255, 255, 0.45)";
pub const ACCENT: &str = "#0090FF";
pub const ACCENT_BG: &str = "rgba(0, 144, 255, 0.15)";
pub const SUCCESS: &str = "#30D158";
pub const SUCCESS_BG: &str = "rgba(48, 209, 88, 0.15)";
pub const WARNING: &str = "#FFBA18";
pub const WARNING_BG: &str = "rgba(255, 186, 24, 0.15)";
pub const DANGER: &str = "#E5484D";
pub const DANGER_BG: &str = "rgba(229, 72, 77, 0.15)";

pub const RADIUS: u32 = 18;
pub const RADIUS_SM: u32 = 10;
pub const RADIUS_PILL: u32 = 999;
pub const INPUT_HEIGHT: u32 = 42;
pub const ROW_HEIGHT: u32 = 38;
pub const BACK_SIZE: u32 = 36;
pub const ICON_BOX: u32 = 36;
pub const GAP: u32 = 12;
pub const GAP_SM: u32 = 10;
pub const GAP_XS: u32 = 4;
pub const PAD: u32 = 16;
pub const PAD_SM: u32 = 14;
pub const SEGMENT_HEIGHT: u32 = 34;

/// 事件列表每页条数（大列表分页，避免长屏拖动）
pub const EVENT_PAGE_SIZE: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Primary,
    Secondary,
    Quiet,
    Danger,
    AccentQuiet,
}

/// 纯样式决策：按钮底色 / 文字色 / 描边都不读全局状态
pub fn button_palette(kind: ButtonKind) -> (&'static str, &'static str, &'static str) {
    match kind {
        ButtonKind::Primary => (ACCENT, TEXT, "transparent"),
        ButtonKind::Secondary => (CONTROL, SECONDARY, OUTLINE),
        ButtonKind::Quiet => ("transparent", MUTED, "transparent"),
        ButtonKind::Danger => (DANGER_BG, DANGER, "transparent"),
        ButtonKind::AccentQuiet => (ACCENT_BG, ACCENT, "transparent"),
    }
}

/// 状态三态配色：成功 / 失败 / 进行中，面板常驻不再只靠一个标记换色
pub fn status_palette(kind: StatusKind) -> (&'static str, &'static str, &'static str) {
    match kind {
        StatusKind::Success => (SUCCESS_BG, SUCCESS, OUTLINE),
        StatusKind::Error => (DANGER_BG, DANGER, OUTLINE),
        StatusKind::Info => (WARNING_BG, WARNING, OUTLINE),
    }
}

/// 分段控制器：选中项用高亮药丸，未选中项保持透明（药丸滑块观感）
pub fn segment_palette(active: bool) -> (&'static str, &'static str) {
    if active {
        (CONTROL_ACTIVE, TEXT)
    } else {
        ("transparent", MUTED)
    }
}

pub fn badge_palette(accented: bool) -> (&'static str, &'static str) {
    if accented {
        (ACCENT_BG, ACCENT)
    } else {
        (CONTROL, SECONDARY)
    }
}

/// 列表行：选中态用 accent 描边，未选中用常规描边
pub fn row_outline(selected: bool) -> (&'static str, &'static str) {
    if selected {
        (ACCENT, ACCENT_BG)
    } else {
        (OUTLINE, CONTROL)
    }
}

pub fn font_title() -> u32 {
    19
}

pub fn font_section() -> u32 {
    20
}

pub fn font_card_title() -> u32 {
    15
}

pub fn font_body() -> u32 {
    14
}

pub fn font_label() -> u32 {
    13
}

pub fn font_hint() -> u32 {
    12
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use super::*;

    #[test]
    fn 按钮语义色互不混用() {
        assert_eq!(button_palette(ButtonKind::Primary).0, ACCENT);
        assert_eq!(button_palette(ButtonKind::Danger).0, DANGER_BG);
        assert_eq!(button_palette(ButtonKind::Danger).1, DANGER);
        assert_eq!(button_palette(ButtonKind::Quiet).0, "transparent");
        assert_eq!(button_palette(ButtonKind::AccentQuiet).0, ACCENT_BG);
    }

    #[test]
    fn 状态三态各有独立底色与前景色() {
        let success = status_palette(StatusKind::Success);
        let error = status_palette(StatusKind::Error);
        let info = status_palette(StatusKind::Info);
        assert_eq!(success.1, SUCCESS);
        assert_eq!(error.1, DANGER);
        assert_eq!(info.1, WARNING);
        assert_ne!(success.0, error.0);
        assert_ne!(info.0, success.0);
    }

    #[test]
    fn 分段控制器选中态才有底色() {
        assert_eq!(segment_palette(true).0, CONTROL_ACTIVE);
        assert_eq!(segment_palette(false).0, "transparent");
        assert_eq!(segment_palette(true).1, TEXT);
        assert_eq!(segment_palette(false).1, MUTED);
    }

    #[test]
    fn 选中行描边使用强调色() {
        assert_eq!(row_outline(true).0, ACCENT);
        assert_ne!(row_outline(true).0, row_outline(false).0);
    }

    #[test]
    fn 圆角与尺寸符合ClaraLight规范() {
        assert_eq!(RADIUS, 18);
        assert_eq!(RADIUS_SM, 10);
        assert_eq!(BACK_SIZE, 36);
        assert_eq!(EVENT_PAGE_SIZE, 8);
        assert_eq!(SEGMENT_HEIGHT, 34);
    }
}
