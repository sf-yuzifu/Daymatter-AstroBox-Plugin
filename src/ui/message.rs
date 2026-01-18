use crate::astrobox::psys_host::{self, device, interconnect, register, thirdpartyapp, timer};
use std::time::Duration;
use super::state::*;
use super::build::build_main_ui;

pub fn show_message(msg: &str, is_success: bool) {
    let (root_id, old_timer_id) = {
        let mut state = ui_state()
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        
        let old_timer_id = state.message_timer_id;
        
        state.error_message = Some(msg.to_string());
        state.is_success_message = is_success;
        
        (state.root_element_id.clone(), old_timer_id)
    };
    
    if let Some(root_id) = root_id {
        let ui = build_main_ui();
        psys_host::ui::render(&root_id, ui);
    }
    
    wit_bindgen::block_on(async move {
        if let Some(timer_id) = old_timer_id {
            let _ = timer::clear_timer(timer_id).await;
        }
        
        let timer_id = timer::set_timeout(3000, "hide_message").await;
        
        let mut state = ui_state()
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.message_timer_id = Some(timer_id);
    });
}

pub fn hide_message() {
    let root_id: Option<String>;
    {
        let mut state = ui_state()
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.error_message = None;
        state.message_timer_id = None;
        root_id = state.root_element_id.clone();
    }
    
    if let Some(root_id) = root_id {
        let ui = build_main_ui();
        psys_host::ui::render(&root_id, ui);
    }
}

pub fn show_error_message(msg: &str) {
    show_message(msg, false);
}

pub fn show_success_message(msg: &str) {
    show_message(msg, true);
}

pub async fn check_device() -> Option<String> {
    let device_list = device::get_connected_device_list().await;
    if let Some(device) = device_list.first() {
        tracing::info!("device: {:?}", device_list);
        let device_addr = device.addr.clone();
        tracing::info!("device_addr: {:?}", device_addr);
        Some(device_addr)
    } else {
        show_error_message("未找到设备");
        None
    }
}

pub async fn check_app_version(device_addr: &str) -> bool {
    let app_list = thirdpartyapp::get_thirdparty_app_list(device_addr).await;

    if let Ok(apps) = app_list {
        tracing::info!("app: {:?}", apps);
        let app = apps.iter().find(|app: &&thirdpartyapp::AppInfo| {
            app.package_name == "com.yzf.daymatter"
        });
        if let Some(app) = app {
            if app.version_code >= 10400 {
                let _ = thirdpartyapp::launch_qa(device_addr, app, "/index").await;
                std::thread::sleep(Duration::from_secs(2));
                true
            } else {
                show_error_message("请先安装倒数日快应用的新版本！");
                false
            }
        } else {
            show_error_message("请先安装倒数日快应用");
            false
        }
    } else {
        show_error_message("获取应用列表失败");
        false
    }
}

pub async fn send_to_daymatter(device_addr: &str, payload: &str) -> bool {
    ensure_interconnect_registered(device_addr).await;
    let result = interconnect::send_qaic_message(device_addr, "com.yzf.daymatter", payload).await;
    if let Ok(_) = result {
        true
    } else {
        show_error_message("发送失败，请重试");
        false
    }
}

async fn ensure_interconnect_registered(device_addr: &str) {
    let _ = register::register_interconnect_recv(
        device_addr,
        "com.yzf.daymatter",
    )
    .await;
}
