use wit_bindgen::FutureReader;
use serde_json::Value;

use crate::exports::astrobox::psys_plugin::{
    event::{self, EventType},
    lifecycle,
};

pub mod logger;
pub mod ui;

wit_bindgen::generate!({
    path: "wit",
    world: "psys-world",
    generate_all,
});

fn extract_payload_text(payload: &str) -> String {
    if let Ok(json) = serde_json::from_str::<Value>(payload) {
        if let Some(text) = json.get("payloadText").and_then(|v| v.as_str()) {
            return text.to_string();
        }
        if let Some(payload_value) = json.get("payload") {
            if let Some(text) = payload_value.as_str() {
                return text.to_string();
            }
            return payload_value.to_string();
        }
    }
    payload.to_string()
}

struct MyPlugin;

impl event::Guest for MyPlugin {
    #[allow(async_fn_in_trait)]
    fn on_event(event_type: EventType, event_payload: _rt::String) -> FutureReader<String> {
        let (writer, reader) = wit_future::new::<String>(|| "".to_string());

        match event_type {
            EventType::PluginMessage => {}
            EventType::InterconnectMessage => {
                ui::handle_interconnect_message(&event_payload);
            }
            EventType::DeviceAction => {}
            EventType::ProviderAction => {}
            EventType::DeeplinkAction => {}
            EventType::TransportPacket => {}
            EventType::Timer => {
                let payload = extract_payload_text(&event_payload);
                if payload == "hide_message" {
                    ui::hide_message();
                }
            }
        };

        tracing::info!("event_type: {:?}, event_payload: {}", event_type, event_payload);

        wit_bindgen::spawn(async move {
            let _ = writer.write("".to_string()).await;
        });

        reader
    }

    fn on_ui_event(
        event_id: _rt::String,
        event_type: event::Event,
        event_payload: _rt::String,
    ) -> wit_bindgen::rt::async_support::FutureReader<_rt::String> {
        let (writer, reader) = wit_future::new::<String>(|| "".to_string());

        ui::ui_event_processor(event_type, &event_id, &event_payload);

        wit_bindgen::spawn(async move {
            let _ = writer.write("".to_string()).await;
        });

        reader
    }

    fn on_ui_render(element_id: _rt::String) -> wit_bindgen::rt::async_support::FutureReader<()> {
        let (writer, reader) = wit_future::new::<()>(|| ());

        ui::render_main_ui(&element_id);

        wit_bindgen::spawn(async move {
            let _ = writer.write(()).await;
        });

        reader
    }

    fn on_card_render(card_id: _rt::String) -> wit_bindgen::rt::async_support::FutureReader<()> {
        let (writer, reader) = wit_future::new::<()>(|| ());

        // 这里可以实现卡片渲染逻辑
        tracing::info!("Card render requested for: {}", card_id);

        wit_bindgen::spawn(async move {
            let _ = writer.write(()).await;
        });

        reader
    }
}

impl lifecycle::Guest for MyPlugin {
    #[allow(async_fn_in_trait)]
    fn on_load() -> () {
        logger::init();
        tracing::info!("Hello AstroBox V2 Plugin!");
    }
}

export!(MyPlugin);
