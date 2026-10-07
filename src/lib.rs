//! AstroBox 插件入口（API Level 4 / wasi_version 3）。
//!
//! 绑定与 guest 实现只在 wasm32 编译，纯逻辑层（`ui::state` / `ui::dispatch` / `ui::view`
//! / `ui::protocol` / `ui::validation`）不带宿主依赖，因此 `cargo test` 可在 host 上直接跑，
//! UI 结构、文案、配色与协议编解码都能脱离 AstroBox 断言（E-16）。

#[cfg(target_arch = "wasm32")]
wit_bindgen::generate!({
    path: "wit",
    world: "psys-world-v4",
    generate_all,
});

pub mod logger;
pub mod ui;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod cross_protocol_tests;

#[cfg(target_arch = "wasm32")]
mod guest;

#[cfg(target_arch = "wasm32")]
pub use guest::MyPlugin;

#[cfg(target_arch = "wasm32")]
export!(MyPlugin);
