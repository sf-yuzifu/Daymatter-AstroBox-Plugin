//! 插件 UI 分层：纯逻辑层（状态 / 事件表 / 视图模型 / 协议 / 校验）可在 host 上跑测试，
//! 图片处理也可在host验证；宿主层（渲染 / 事件处理 / 消息）只在 wasm32 编译。

pub mod dispatch;
pub mod backup;
pub mod lunar;
pub mod protocol;
pub mod state;
pub mod theme;
pub mod validation;
pub mod view;

#[cfg(target_arch = "wasm32")]
mod build;
#[cfg(target_arch = "wasm32")]
mod event_handler;
#[cfg(any(target_arch = "wasm32", test))]
mod image;
#[cfg(target_arch = "wasm32")]
mod message;
#[cfg(target_arch = "wasm32")]
mod widgets;

#[cfg(target_arch = "wasm32")]
pub use build::render_main_ui;
#[cfg(target_arch = "wasm32")]
pub use event_handler::{handle_interconnect_message, handle_timer_event, ui_event_processor};
#[cfg(target_arch = "wasm32")]
pub use message::hide_message;
#[cfg(target_arch = "wasm32")]
pub use message::sync_today;
