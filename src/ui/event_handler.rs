use crate::astrobox::psys_host::{self, ui};
use serde_json::Value;
use super::state::*;
use super::message::*;
use super::validation::*;
use super::build::build_main_ui;

pub const TAB_CHANGE_EVENT: &str = "tab_change";
pub const EVENT_NAME_INPUT_EVENT: &str = "event_name_input";
pub const EVENT_TIME_INPUT_EVENT: &str = "event_time_input";
pub const ON_INDEX_CHANGE_EVENT: &str = "on_index_change";
pub const ON_INDEX_YES_EVENT: &str = "on_index_yes";
pub const ON_INDEX_NO_EVENT: &str = "on_index_no";
pub const IF_STARING_DAY_CHANGE_EVENT: &str = "if_staring_day_change";
pub const IF_STARING_DAY_YES_EVENT: &str = "if_staring_day_yes";
pub const IF_STARING_DAY_NO_EVENT: &str = "if_staring_day_no";
pub const MODIFY_EVENT_NAME_INPUT_EVENT: &str = "modify_event_name_input";
pub const MODIFY_EVENT_TIME_INPUT_EVENT: &str = "modify_event_time_input";
pub const MODIFY_ON_INDEX_CHANGE_EVENT: &str = "modify_on_index_change";
pub const MODIFY_ON_INDEX_YES_EVENT: &str = "modify_on_index_yes";
pub const MODIFY_ON_INDEX_NO_EVENT: &str = "modify_on_index_no";
pub const MODIFY_IF_STARING_DAY_CHANGE_EVENT: &str = "modify_if_staring_day_change";
pub const MODIFY_IF_STARING_DAY_YES_EVENT: &str = "modify_if_staring_day_yes";
pub const MODIFY_IF_STARING_DAY_NO_EVENT: &str = "modify_if_staring_day_no";
pub const ADD_EVENT_BUTTON_EVENT: &str = "add_event_button";
pub const GET_EVENTS_BUTTON_EVENT: &str = "get_events_button";
pub const CHANGE_EVENT_BUTTON_EVENT: &str = "change_event_button";
pub const DELETE_EVENT_BUTTON_EVENT: &str = "delete_event_button";
pub const SELECT_EVENT_DROPDOWN_EVENT: &str = "select_event_dropdown";
pub const TAB_ADD_EVENT: &str = "tab_add_event";
pub const TAB_MODIFY_EVENT: &str = "tab_modify_event";
pub const TAB_DELETE_EVENT: &str = "tab_delete_event";
pub const HIDE_ERROR_EVENT: &str = "hide_error";
pub const BUTTON_MOUSE_LEAVE: &str = "button_mouse_leave";

pub fn handle_interconnect_message(payload: &str) {
    tracing::info!("收到手环端消息: {}", payload);

    if let Ok(json) = serde_json::from_str::<Value>(payload) {
        let data_json = if let Some(payload_text) = json.get("payloadText").and_then(|v| v.as_str())
        {
            tracing::info!("从 payloadText 解析数据");
            payload_text
        } else {
            tracing::info!("直接使用 payload 解析数据");
            payload
        };

        if let Ok(data_json) = serde_json::from_str::<Value>(data_json) {
            if let Some(data) = data_json.get("data") {
                if let Some(events_array) = data.as_array() {
                    tracing::info!("解析到 {} 个事件", events_array.len());
                    let mut all_events = Vec::new();
                    for event in events_array {
                        if let Some(event_obj) = event.as_object() {
                            let name = event_obj
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let date = event_obj
                                .get("date")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let on_index = event_obj
                                .get("on_index")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            let if_staring_day = event_obj
                                .get("IFStaringDay")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);

                            tracing::info!(
                                "事件: name={}, date={}, on_index={}, if_staring_day={}",
                                name,
                                date,
                                on_index,
                                if_staring_day
                            );
                            all_events.push(EventData {
                                name,
                                time: date,
                                on_index,
                                if_staring_day,
                            });
                        }
                    }

                    let root_id: Option<String>;
                    {
                        let mut state = ui_state()
                            .write()
                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                        state.all_events = all_events;
                        state.has_fetched_events = true;
                        state.error_message = Some("获取手环端数据成功！".to_string());
                        state.is_success_message = true;
                        tracing::info!(
                            "更新状态: all_events.len={}, error_message={:?}",
                            state.all_events.len(),
                            state.error_message
                        );
                        root_id = state.root_element_id.clone();
                        tracing::info!("root_element_id: {:?}", root_id);
                    }
                    if let Some(root_id) = root_id {
                        let ui = build_main_ui();
                        psys_host::ui::render(&root_id, ui);
                        tracing::info!("UI已重新渲染");
                    } else {
                        tracing::warn!("root_element_id 为 None，无法重新渲染UI");
                    }
                } else {
                    tracing::warn!("data 不是数组");
                }
            } else {
                tracing::warn!("JSON 中没有 data 字段");
            }
        } else {
            tracing::warn!("JSON 解析失败");
        }
    } else {
        tracing::warn!("JSON 解析失败");
    }
}

fn handle_input_event(event: &str, value: &str) {
    let mut state = ui_state()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let parsed_value = if let Ok(json) = serde_json::from_str::<serde_json::Value>(value) {
        json.get("value")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    } else {
        value.to_string()
    };

    match event {
        EVENT_NAME_INPUT_EVENT => {
            state.event_data.name = parsed_value;
            state.error_message = None;
        }
        EVENT_TIME_INPUT_EVENT => {
            state.event_data.time = parsed_value;
            state.error_message = None;
        }
        MODIFY_EVENT_NAME_INPUT_EVENT => {
            state.modify_event_data.name = parsed_value;
        }
        MODIFY_EVENT_TIME_INPUT_EVENT => {
            state.modify_event_data.time = parsed_value;
        }
        _ => {
            tracing::info!("未处理的事件类型");
        }
    }
}

fn handle_dropdown_event(event: &str, value: &str) {
    let root_id: Option<String>;
    {
        let mut state = ui_state()
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let parsed_value = if let Ok(json) = serde_json::from_str::<serde_json::Value>(value) {
            json.get("value")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        } else {
            value.to_string()
        };

        root_id = state.root_element_id.clone();

        match event {
            ON_INDEX_CHANGE_EVENT => {
                state.event_data.on_index = parsed_value == "是";
            }
            IF_STARING_DAY_CHANGE_EVENT => {
                state.event_data.if_staring_day = parsed_value == "是";
            }
            MODIFY_ON_INDEX_CHANGE_EVENT => {
                state.modify_event_data.on_index = parsed_value == "是";
            }
            MODIFY_IF_STARING_DAY_CHANGE_EVENT => {
                state.modify_event_data.if_staring_day = parsed_value == "是";
            }
            SELECT_EVENT_DROPDOWN_EVENT => {
                if let Some(space_idx) = parsed_value.find("　") {
                    if let Ok(index) = parsed_value[..space_idx].parse::<usize>() {
                        state.selected_event_index = Some(index - 1);
                        if let Some(event_data) = state.all_events.get(index - 1).cloned() {
                            state.modify_event_data.name = event_data.name.clone();
                            state.modify_event_data.time = event_data.time;
                            state.modify_event_data.on_index = event_data.on_index;
                            state.modify_event_data.if_staring_day = event_data.if_staring_day;
                            state.selected_event_name = Some(parsed_value);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(root_id) = root_id {
        let ui = build_main_ui();
        psys_host::ui::render(&root_id, ui);
    }
}

fn handle_button_click(event: &str) {
    match event {
        ON_INDEX_YES_EVENT => {
            let root_id: Option<String>;
            {
                let mut state = ui_state()
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.event_data.on_index = true;
                root_id = state.root_element_id.clone();
            }
            if let Some(root_id) = root_id {
                let ui = build_main_ui();
                psys_host::ui::render(&root_id, ui);
            }
        }
        ON_INDEX_NO_EVENT => {
            let root_id: Option<String>;
            {
                let mut state = ui_state()
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.event_data.on_index = false;
                root_id = state.root_element_id.clone();
            }
            if let Some(root_id) = root_id {
                let ui = build_main_ui();
                psys_host::ui::render(&root_id, ui);
            }
        }
        IF_STARING_DAY_YES_EVENT => {
            let root_id: Option<String>;
            {
                let mut state = ui_state()
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.event_data.if_staring_day = true;
                root_id = state.root_element_id.clone();
            }
            if let Some(root_id) = root_id {
                let ui = build_main_ui();
                psys_host::ui::render(&root_id, ui);
            }
        }
        IF_STARING_DAY_NO_EVENT => {
            let root_id: Option<String>;
            {
                let mut state = ui_state()
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.event_data.if_staring_day = false;
                root_id = state.root_element_id.clone();
            }
            if let Some(root_id) = root_id {
                let ui = build_main_ui();
                psys_host::ui::render(&root_id, ui);
            }
        }
        MODIFY_ON_INDEX_YES_EVENT => {
            let root_id: Option<String>;
            {
                let mut state = ui_state()
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.modify_event_data.on_index = true;
                root_id = state.root_element_id.clone();
            }
            if let Some(root_id) = root_id {
                let ui = build_main_ui();
                psys_host::ui::render(&root_id, ui);
            }
        }
        MODIFY_ON_INDEX_NO_EVENT => {
            let root_id: Option<String>;
            {
                let mut state = ui_state()
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.modify_event_data.on_index = false;
                root_id = state.root_element_id.clone();
            }
            if let Some(root_id) = root_id {
                let ui = build_main_ui();
                psys_host::ui::render(&root_id, ui);
            }
        }
        MODIFY_IF_STARING_DAY_YES_EVENT => {
            let root_id: Option<String>;
            {
                let mut state = ui_state()
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.modify_event_data.if_staring_day = true;
                root_id = state.root_element_id.clone();
            }
            if let Some(root_id) = root_id {
                let ui = build_main_ui();
                psys_host::ui::render(&root_id, ui);
            }
        }
        MODIFY_IF_STARING_DAY_NO_EVENT => {
            let root_id: Option<String>;
            {
                let mut state = ui_state()
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.modify_event_data.if_staring_day = false;
                root_id = state.root_element_id.clone();
            }
            if let Some(root_id) = root_id {
                let ui = build_main_ui();
                psys_host::ui::render(&root_id, ui);
            }
        }
        ADD_EVENT_BUTTON_EVENT => {
            let (event_name, event_time, on_index, if_staring_day) = {
                let state = ui_state()
                    .read()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                (
                    state.event_data.name.clone(),
                    state.event_data.time.clone(),
                    state.event_data.on_index,
                    state.event_data.if_staring_day,
                )
            };

            let error_message = if event_name.is_empty() {
                Some("事件名称不能为空".to_string())
            } else if let Err(err) = validate_date_format(&event_time) {
                Some(err.to_string())
            } else {
                None
            };

            if let Some(msg) = error_message {
                show_error_message(&msg);
            } else {
                {
                    let mut state = ui_state()
                        .write()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    state.error_message = None;
                }

                tracing::info!(
                    "添加事件: name={}, time={}, on_index={}, if_staring_day={}",
                    event_name,
                    event_time,
                    on_index,
                    if_staring_day
                );

                show_message("正在发送，请稍等···", false);

                let event_name_clone = event_name.clone();
                let event_time_clone = event_time.clone();
                let on_index_clone = on_index;
                let if_staring_day_clone = if_staring_day;

                wit_bindgen::block_on(async move {
                    if let Some(device_addr) = check_device().await {
                        if check_app_version(&device_addr).await {
                            let payload = format!(
                                r#"{{"type":"addEvent","name":"{}","date":"{}","on_index":{},"IFStaringDay":{}}}"#,
                                event_name_clone, event_time_clone, on_index_clone, if_staring_day_clone
                            );

                            if send_to_daymatter(&device_addr, &payload).await {
                                let root_id: Option<String>;
                                {
                                    let mut state = ui_state()
                                        .write()
                                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                                    
                                    state.all_events.clear();
                                    state.has_fetched_events = false;
                                    
                                    root_id = state.root_element_id.clone();
                                }
                                
                                show_success_message("发送成功！");
                                
                                if let Some(root_id) = root_id {
                                    let ui = build_main_ui();
                                    psys_host::ui::render(&root_id, ui);
                                }
                            }
                        }
                    }
                });
            }
        }
        GET_EVENTS_BUTTON_EVENT => {
            show_message("正在发送，请稍等···", false);

            wit_bindgen::block_on(async move {
                if let Some(device_addr) = check_device().await {
                    if check_app_version(&device_addr).await {
                        let payload = r#"{"type":"getAllEvent"}"#;

                        if send_to_daymatter(&device_addr, &payload).await {
                            show_success_message("获取手环端数据成功！");
                        }
                    }
                }
            });
        }
        CHANGE_EVENT_BUTTON_EVENT => {
            let (event_name, event_time, on_index, if_staring_day, selected_index) = {
                let state = ui_state()
                    .read()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                (
                    state.modify_event_data.name.clone(),
                    state.modify_event_data.time.clone(),
                    state.modify_event_data.on_index,
                    state.modify_event_data.if_staring_day,
                    state.selected_event_index,
                )
            };

            let error_message = if selected_index.is_none() {
                Some("请选择你要修改的事件！".to_string())
            } else if event_name.is_empty() {
                Some("事件名称不能为空".to_string())
            } else if let Err(err) = validate_date_format(&event_time) {
                Some(err.to_string())
            } else {
                None
            };

            if let Some(msg) = error_message {
                show_error_message(&msg);
            } else {
                {
                    let mut state = ui_state()
                        .write()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    state.error_message = None;
                }

                tracing::info!(
                    "修改事件: name={}, time={}, on_index={}, if_staring_day={}, index={:?}",
                    event_name,
                    event_time,
                    on_index,
                    if_staring_day,
                    selected_index
                );

                show_message("正在发送，请稍等···", false);

                let event_name_clone = event_name.clone();
                let event_time_clone = event_time.clone();
                let on_index_clone = on_index;
                let if_staring_day_clone = if_staring_day;
                let index_clone = selected_index.unwrap_or(0);

                wit_bindgen::block_on(async move {
                    if let Some(device_addr) = check_device().await {
                        if check_app_version(&device_addr).await {
                            let payload = format!(
                                r#"{{"type":"changeEvent","name":"{}","date":"{}","on_index":{},"IFStaringDay":{},"index":{}}}"#,
                                event_name_clone, event_time_clone, on_index_clone, if_staring_day_clone, index_clone
                            );

                            if send_to_daymatter(&device_addr, &payload).await {
                                let root_id: Option<String>;
                                {
                                    let mut state = ui_state()
                                        .write()
                                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                                    state.error_message = Some("发送成功！".to_string());
                                    state.is_success_message = true;
                                    if let Some(index) = state.selected_event_index {
                                        if let Some(event) = state.all_events.get_mut(index) {
                                            event.name = event_name_clone.clone();
                                            event.time = event_time_clone.clone();
                                            event.on_index = on_index_clone;
                                            event.if_staring_day = if_staring_day_clone;
                                        }
                                        state.selected_event_name =
                                            Some(format!("{}　{}", index + 1, event_name_clone));
                                    }
                                    root_id = state.root_element_id.clone();
                                }
                                if let Some(root_id) = root_id {
                                    let ui = build_main_ui();
                                    psys_host::ui::render(&root_id, ui);
                                }
                            }
                        }
                    }
                });
            }
        }
        DELETE_EVENT_BUTTON_EVENT => {
            let selected_index = {
                let state = ui_state()
                    .read()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.selected_event_index
            };

            let error_message = if selected_index.is_none() {
                Some("请选择你要删除的事件！".to_string())
            } else {
                None
            };

            if let Some(msg) = error_message {
                show_error_message(&msg);
            } else {
                show_message("正在发送，请稍等···", false);

                let index_clone = selected_index.unwrap_or(0);

                wit_bindgen::block_on(async move {
                    if let Some(device_addr) = check_device().await {
                        if check_app_version(&device_addr).await {
                            let payload = format!(
                                r#"{{"type":"deleteEvent","index":{}}}"#,
                                index_clone
                            );

                            if send_to_daymatter(&device_addr, &payload).await {
                                let root_id: Option<String>;
                                {
                                    let mut state = ui_state()
                                        .write()
                                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                                    state.error_message = Some("发送成功！".to_string());
                                    state.is_success_message = true;
                                    if let Some(index) = state.selected_event_index {
                                        state.all_events.remove(index);
                                        state.selected_event_index = None;
                                        state.selected_event_name = None;
                                    }
                                    root_id = state.root_element_id.clone();
                                }
                                if let Some(root_id) = root_id {
                                    let ui = build_main_ui();
                                    psys_host::ui::render(&root_id, ui);
                                }
                            }
                        }
                    }
                });
            }
        }
        HIDE_ERROR_EVENT => {
            let root_id: Option<String>;
            {
                let mut state = ui_state()
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.error_message = None;
                root_id = state.root_element_id.clone();
            }
            if let Some(root_id) = root_id {
                let ui = build_main_ui();
                psys_host::ui::render(&root_id, ui);
            }
        }
        TAB_ADD_EVENT => {
            let root_id: Option<String>;
            {
                let mut state = ui_state()
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.current_tab = EventType::AddEvent;
                root_id = state.root_element_id.clone();
            }
            if let Some(root_id) = root_id {
                let ui = build_main_ui();
                psys_host::ui::render(&root_id, ui);
            }
        }
        TAB_MODIFY_EVENT => {
            let root_id: Option<String>;
            {
                let mut state = ui_state()
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.current_tab = EventType::ModifyEvent;
                root_id = state.root_element_id.clone();
            }
            if let Some(root_id) = root_id {
                let ui = build_main_ui();
                psys_host::ui::render(&root_id, ui);
            }
        }
        TAB_DELETE_EVENT => {
            let root_id: Option<String>;
            {
                let mut state = ui_state()
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.current_tab = EventType::DeleteEvent;
                root_id = state.root_element_id.clone();
            }
            if let Some(root_id) = root_id {
                let ui = build_main_ui();
                psys_host::ui::render(&root_id, ui);
            }
        }
        BUTTON_MOUSE_LEAVE => {
            let root_id: Option<String>;
            {
                let mut state = ui_state()
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.hovered_button = None;
                root_id = state.root_element_id.clone();
            }
            if let Some(root_id) = root_id {
                let ui = build_main_ui();
                psys_host::ui::render(&root_id, ui);
            }
        }
        _ => {}
    }
}

fn handle_mouse_enter(event: &str) {
    let root_id: Option<String>;
    {
        let mut state = ui_state()
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.hovered_button = Some(event.to_string());
        root_id = state.root_element_id.clone();
    }
    if let Some(root_id) = root_id {
        let ui = build_main_ui();
        psys_host::ui::render(&root_id, ui);
    }
}

fn handle_mouse_leave(_event: &str) {
    let root_id: Option<String>;
    {
        let mut state = ui_state()
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.hovered_button = None;
        root_id = state.root_element_id.clone();
    }
    if let Some(root_id) = root_id {
        let ui = build_main_ui();
        psys_host::ui::render(&root_id, ui);
    }
}

pub fn ui_event_processor(evtype: ui::Event, event: &str, event_payload: &str) {
    let evtype_str = format!("{:?}", evtype);
    tracing::info!(
        "接收到事件: 类型={}, 名称={}, 载荷={}",
        evtype_str,
        event,
        event_payload
    );
    match evtype {
        ui::Event::Click => {
            handle_button_click(event);
        }
        ui::Event::Change => {
            if event.starts_with("event_") || event.starts_with("modify_") {
                handle_input_event(event, event_payload);
            } else {
                handle_dropdown_event(event, event_payload);
            }
        }
        ui::Event::MouseEnter => {
            handle_mouse_enter(event);
        }
        ui::Event::MouseLeave => {
            handle_mouse_leave(event);
        }
        _ => {}
    }
}
