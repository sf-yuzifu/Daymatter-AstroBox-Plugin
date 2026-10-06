//! 宿主导出实现（API Level 4）：全部为 `async fn`，可直接 await 宿主接口，
//! 不再需要 `wit_future::new()` + spawn 样板，也不用在 `on-load` 里 `block_on`（E-04 / E-21）。

use crate::astrobox::psys_host_v4::ui as ui_event;
use crate::exports::astrobox::psys_plugin_v4::{event, lifecycle};

pub struct MyPlugin;

impl lifecycle::Guest for MyPlugin {
    async fn on_load() {
        crate::logger::init();
        tracing::info!("倒数日 AstroBox 插件已加载（API Level 4）");
        crate::ui::sync_today().await;
    }
}

impl event::Guest for MyPlugin {
    async fn on_event(event_type: event::EventType, event_payload: String) -> String {
        tracing::info!("event_type: {:?}, payload_len: {}", event_type, event_payload.len());
        match event_type {
            event::EventType::InterconnectMessage => {
                crate::ui::handle_interconnect_message(&event_payload).await;
            }
            event::EventType::Timer => {
                crate::ui::handle_timer_event(&event_payload).await;
            }
            _ => {}
        }
        String::new()
    }

    async fn on_ui_event(event_id: String, event_type: ui_event::Event, event_payload: String) -> String {
        crate::ui::ui_event_processor(event_type, &event_id, &event_payload).await;
        String::new()
    }

    async fn on_ui_render(element_id: String) {
        crate::ui::render_main_ui(&element_id);
    }

    async fn on_card_render(card_id: String) {
        tracing::info!("未注册的卡片: {}", card_id);
    }
}
