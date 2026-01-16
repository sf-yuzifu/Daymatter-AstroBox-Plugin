use crate::astrobox::psys_host::{self, device, interconnect, thirdpartyapp, ui};
use serde_json::Value;
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// 处理从手环端接收到的消息
pub fn handle_interconnect_message(payload: &str) {
    tracing::info!("收到手环端消息: {}", payload);

    if let Ok(json) = serde_json::from_str::<Value>(payload) {
        // 尝试从 payloadText 字段获取数据
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

// 确保已注册接收 Interconnect 消息
async fn ensure_interconnect_registered(device_addr: &str) {
    use crate::astrobox::psys_host::register;

    let result = register::register_interconnect_recv(device_addr, "com.yzf.daymatter").await;
    match result {
        Ok(_) => {
            tracing::info!(
                "成功注册接收倒数日应用的 Interconnect 消息: {}",
                device_addr
            );
        }
        Err(e) => {
            tracing::error!(
                "注册接收倒数日应用的 Interconnect 消息失败: {:?}, 设备: {}",
                e,
                device_addr
            );
        }
    }
}

// 事件类型枚举
#[derive(Clone, Copy, PartialEq)]
pub enum EventType {
    AddEvent,
    ModifyEvent,
    DeleteEvent,
}

// 事件数据结构
#[derive(Debug, Clone)]
struct EventData {
    name: String,
    time: String,
    on_index: bool,
    if_staring_day: bool,
}

impl Default for EventData {
    fn default() -> Self {
        EventData {
            name: String::new(),
            time: get_current_date(),
            on_index: false,
            if_staring_day: false,
        }
    }
}

// UI状态管理
struct UiState {
    root_element_id: Option<String>,
    current_tab: EventType,
    event_data: EventData,
    modify_event_data: EventData,
    all_events: Vec<EventData>, // 存储所有事件的完整数据
    selected_event_index: Option<usize>,
    selected_event_name: Option<String>, // 保存选中的事件名称
    has_fetched_events: bool,            // 是否已经获取了手环端数据
    hovered_button: Option<String>,      // 跟踪当前悬停的按钮
    error_message: Option<String>,       // 错误提示消息
    is_success_message: bool,             // 是否为成功消息（用于区分颜色）
}

static UI_STATE: OnceLock<RwLock<UiState>> = OnceLock::new();

fn ui_state() -> &'static RwLock<UiState> {
    UI_STATE.get_or_init(|| {
        RwLock::new(UiState {
            root_element_id: None,
            current_tab: EventType::AddEvent,
            event_data: EventData::default(),
            modify_event_data: EventData {
                name: String::new(),
                time: String::new(),
                on_index: false,
                if_staring_day: false,
            },
            all_events: Vec::new(),
            selected_event_index: None,
            selected_event_name: None,
            has_fetched_events: false,
            hovered_button: None,
            error_message: None,
            is_success_message: false,
        })
    })
}

// 事件常量定义
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

// 辅助函数：显示消息
fn show_message(msg: &str, is_success: bool) {
    let root_id: Option<String>;
    {
        let mut state = ui_state()
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.error_message = Some(msg.to_string());
        state.is_success_message = is_success;
        root_id = state.root_element_id.clone();
    }
    if let Some(root_id) = root_id {
        let ui = build_main_ui();
        psys_host::ui::render(&root_id, ui);
    }
}

fn show_error_message(msg: &str) {
    show_message(msg, false);
}

fn show_success_message(msg: &str) {
    show_message(msg, true);
}

// 辅助函数：检查设备并返回设备地址
async fn check_device() -> Option<String> {
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

// 辅助函数：检查应用版本
async fn check_app_version(device_addr: &str) -> bool {
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

// 辅助函数：发送消息到倒数日应用
async fn send_to_daymatter(device_addr: &str, payload: &str) -> bool {
    ensure_interconnect_registered(device_addr).await;
    let result = interconnect::send_qaic_message(device_addr, "com.yzf.daymatter", payload).await;
    if let Ok(_) = result {
        true
    } else {
        show_error_message("发送失败，请重试");
        false
    }
}

// 日期验证错误类型
#[derive(Debug)]
enum DateValidationError {
    Empty,
    InvalidFormat,
    InvalidYear,
    InvalidMonth,
    InvalidDay,
}

impl std::fmt::Display for DateValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DateValidationError::Empty => write!(f, "时间不能为空"),
            DateValidationError::InvalidFormat => write!(f, "时间格式必须为 YYYY-MM-DD"),
            DateValidationError::InvalidYear => write!(f, "年份必须在 1-9999 之间"),
            DateValidationError::InvalidMonth => write!(f, "月份必须在 1-12 之间"),
            DateValidationError::InvalidDay => write!(f, "日期无效"),
        }
    }
}

// 验证时间格式 YYYY-MM-DD，返回具体的错误信息
fn validate_date_format(date_str: &str) -> Result<(), DateValidationError> {
    if date_str.is_empty() {
        return Err(DateValidationError::Empty);
    }

    if date_str.len() != 10 {
        return Err(DateValidationError::InvalidFormat);
    }

    let chars: Vec<char> = date_str.chars().collect();

    if chars[4] != '-' || chars[7] != '-' {
        return Err(DateValidationError::InvalidFormat);
    }

    let year_str = &date_str[0..4];
    let month_str = &date_str[5..7];
    let day_str = &date_str[8..10];

    let year: u32 = match year_str.parse() {
        Ok(y) => y,
        Err(_) => return Err(DateValidationError::InvalidFormat),
    };

    let month: u32 = match month_str.parse() {
        Ok(m) => m,
        Err(_) => return Err(DateValidationError::InvalidFormat),
    };

    let day: u32 = match day_str.parse() {
        Ok(d) => d,
        Err(_) => return Err(DateValidationError::InvalidFormat),
    };

    if year < 1 || year > 9999 {
        return Err(DateValidationError::InvalidYear);
    }

    if month < 1 || month > 12 {
        return Err(DateValidationError::InvalidMonth);
    }

    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => return Err(DateValidationError::InvalidMonth),
    };

    if day < 1 || day > days_in_month {
        return Err(DateValidationError::InvalidDay);
    }

    Ok(())
}

fn is_leap_year(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

// 获取当前日期并格式化为 YYYY-MM-DD
fn get_current_date() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(std::time::Duration::from_secs(0));

    let total_seconds = duration.as_secs();
    let total_days = total_seconds / 86400;

    // 1970-01-01 是 Unix 纪元
    let mut year = 1970;
    let mut remaining_days = total_days;

    while remaining_days > 0 {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days >= days_in_year {
            remaining_days -= days_in_year;
            year += 1;
        } else {
            break;
        }
    }

    let mut month = 1;
    let mut day = remaining_days + 1;

    while month <= 12 {
        let days_in_month = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if is_leap_year(year) {
                    29
                } else {
                    28
                }
            }
            _ => break,
        };

        if day > days_in_month {
            day -= days_in_month;
            month += 1;
        } else {
            break;
        }
    }

    format!("{:04}-{:02}-{:02}", year, month, day)
}

// 输入事件处理
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

// 下拉菜单事件处理
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
                // 解析选择的事件索引
                if let Some(space_idx) = parsed_value.find("　") {
                    if let Ok(index) = parsed_value[..space_idx].parse::<usize>() {
                        state.selected_event_index = Some(index - 1);
                        // 加载选中事件的数据
                        if let Some(event_data) = state.all_events.get(index - 1).cloned() {
                            state.modify_event_data.name = event_data.name.clone();
                            state.modify_event_data.time = event_data.time;
                            state.modify_event_data.on_index = event_data.on_index;
                            state.modify_event_data.if_staring_day = event_data.if_staring_day;
                            // 保存选中的事件名称（包含索引）
                            state.selected_event_name = Some(parsed_value);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // 重新渲染 UI（在锁释放后）
    if let Some(root_id) = root_id {
        let ui = build_main_ui();
        psys_host::ui::render(&root_id, ui);
    }
}

// 按钮点击事件处理
fn handle_button_click(event: &str) {
    match event {
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
                                show_success_message("发送成功！");
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
            // 隐藏错误提示
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
            // 切换到添加事件标签页
            let root_id: Option<String>;
            {
                let mut state = ui_state()
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.current_tab = EventType::AddEvent;
                root_id = state.root_element_id.clone();
                // 作用域结束，自动释放写锁
            }

            // 直接重新渲染UI，避免递归调用
            if let Some(root_id) = root_id {
                let ui = build_main_ui();
                psys_host::ui::render(&root_id, ui);
            }
        }
        TAB_MODIFY_EVENT => {
            // 切换到修改事件标签页
            let root_id: Option<String>;
            {
                let mut state = ui_state()
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.current_tab = EventType::ModifyEvent;
                root_id = state.root_element_id.clone();
                // 作用域结束，自动释放写锁
            }

            // 直接重新渲染UI，避免递归调用
            if let Some(root_id) = root_id {
                let ui = build_main_ui();
                psys_host::ui::render(&root_id, ui);
            }
        }
        TAB_DELETE_EVENT => {
            // 切换到删除事件标签页
            let root_id: Option<String>;
            {
                let mut state = ui_state()
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.current_tab = EventType::DeleteEvent;
                root_id = state.root_element_id.clone();
                // 作用域结束，自动释放写锁
            }

            // 直接重新渲染UI，避免递归调用
            if let Some(root_id) = root_id {
                let ui = build_main_ui();
                psys_host::ui::render(&root_id, ui);
            }
        }
        ON_INDEX_YES_EVENT => {
            // 设置是否显示在主页为是
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
            // 设置是否显示在主页为否
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
            // 设置是否计入起始日为是
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
            // 设置是否计入起始日为否
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
            // 设置修改事件的是否显示在主页为是
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
            // 设置修改事件的是否显示在主页为否
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
            // 设置修改事件的是否计入起始日为是
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
            // 设置修改事件的是否计入起始日为否
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
        _ => {}
    }
}

// 鼠标进入事件处理
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

// 鼠标离开事件处理
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

// 事件处理器
pub fn ui_event_processor(evtype: ui::Event, event: &str, event_payload: &str) {
    // 输出事件类型的原始字符串表示
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
            // 处理输入和下拉菜单的变化
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

// 构建添加事件界面
fn build_add_event_ui(state: &UiState) -> ui::Element {
    let container = ui::Element::new(ui::ElementType::Div, None)
        .flex()
        .flex_direction(ui::FlexDirection::Column);

    // 事件名称输入
    let event_name_label = ui::Element::new(ui::ElementType::P, Some("输入事件名称"))
        .size(16)
        .margin_bottom(8);
    let event_name_input = ui::Element::new(ui::ElementType::Input, Some(&state.event_data.name))
        .on(ui::Event::Change, EVENT_NAME_INPUT_EVENT)
        .radius(8)
        .bg("#2A2A2A")
        .width_full()
        .margin_bottom(8);

    // 事件时间输入
    let event_time_label = ui::Element::new(
        ui::ElementType::P,
        Some("输入事件目标日（格式必须为YYYY-MM-DD）"),
    )
    .size(16)
    .margin_bottom(8);
    let event_time_input = ui::Element::new(ui::ElementType::Input, Some(&state.event_data.time))
        .on(ui::Event::Change, EVENT_TIME_INPUT_EVENT)
        .radius(8)
        .bg("#2A2A2A")
        .width_full()
        .margin_bottom(8);

    // 是否显示在主页
    let on_index_container = ui::Element::new(ui::ElementType::Div, None)
        .relative()
        .margin_bottom(8)
        .height(40)
        .width_full();
    let on_index_label = ui::Element::new(ui::ElementType::P, Some("是否显示在主页"))
        .absolute()
        .size(16)
        .left(0)
        .top(10);
    let on_index_buttons = ui::Element::new(ui::ElementType::Div, None)
        .absolute()
        .flex()
        .flex_direction(ui::FlexDirection::Row)
        .right(0)
        .top(5);
    let on_index_yes = ui::Element::new(ui::ElementType::Button, Some("是"))
        .without_default_styles()
        .on(ui::Event::Click, ON_INDEX_YES_EVENT)
        .on(ui::Event::MouseEnter, ON_INDEX_YES_EVENT)
        .on(ui::Event::MouseLeave, BUTTON_MOUSE_LEAVE)
        .radius(8)
        .padding_top(8)
        .padding_right(25)
        .padding_bottom(8)
        .padding_left(25)
        .bg(if state.event_data.on_index {
            "#424242"
        } else if state.hovered_button.as_deref() == Some(ON_INDEX_YES_EVENT) {
            "#4b4b4b"
        } else {
            "#2A2A2A"
        })
        .margin_right(5);
    let on_index_no = ui::Element::new(ui::ElementType::Button, Some("否"))
        .without_default_styles()
        .on(ui::Event::Click, ON_INDEX_NO_EVENT)
        .on(ui::Event::MouseEnter, ON_INDEX_NO_EVENT)
        .on(ui::Event::MouseLeave, BUTTON_MOUSE_LEAVE)
        .radius(8)
        .padding_top(8)
        .padding_right(25)
        .padding_bottom(8)
        .padding_left(25)
        .bg(if !state.event_data.on_index {
            "#424242"
        } else if state.hovered_button.as_deref() == Some(ON_INDEX_NO_EVENT) {
            "#4b4b4b"
        } else {
            "#2A2A2A"
        });

    // 是否计入起始日
    let if_staring_day_container = ui::Element::new(ui::ElementType::Div, None)
        .relative()
        .margin_bottom(8)
        .height(40)
        .width_full();
    let if_staring_day_label = ui::Element::new(ui::ElementType::P, Some("是否计入起始日"))
        .absolute()
        .size(16)
        .left(0)
        .top(10);
    let if_staring_day_buttons = ui::Element::new(ui::ElementType::Div, None)
        .absolute()
        .flex()
        .flex_direction(ui::FlexDirection::Row)
        .right(0)
        .top(5);
    let if_staring_day_yes = ui::Element::new(ui::ElementType::Button, Some("是"))
        .without_default_styles()
        .on(ui::Event::Click, IF_STARING_DAY_YES_EVENT)
        .on(ui::Event::MouseEnter, IF_STARING_DAY_YES_EVENT)
        .on(ui::Event::MouseLeave, BUTTON_MOUSE_LEAVE)
        .radius(8)
        .padding_top(8)
        .padding_right(25)
        .padding_bottom(8)
        .padding_left(25)
        .bg(if state.event_data.if_staring_day {
            "#424242"
        } else if state.hovered_button.as_deref() == Some(IF_STARING_DAY_YES_EVENT) {
            "#4b4b4b"
        } else {
            "#2A2A2A"
        })
        .margin_right(5);
    let if_staring_day_no = ui::Element::new(ui::ElementType::Button, Some("否"))
        .without_default_styles()
        .on(ui::Event::Click, IF_STARING_DAY_NO_EVENT)
        .on(ui::Event::MouseEnter, IF_STARING_DAY_NO_EVENT)
        .on(ui::Event::MouseLeave, BUTTON_MOUSE_LEAVE)
        .radius(8)
        .padding_top(8)
        .padding_right(25)
        .padding_bottom(8)
        .padding_left(25)
        .bg(if !state.event_data.if_staring_day {
            "#424242"
        } else if state.hovered_button.as_deref() == Some(IF_STARING_DAY_NO_EVENT) {
            "#4b4b4b"
        } else {
            "#2A2A2A"
        });

    // 添加事件按钮
    let add_event_button = ui::Element::new(ui::ElementType::Button, Some("发送"))
        .without_default_styles()
        .on(ui::Event::Click, ADD_EVENT_BUTTON_EVENT)
        .on(ui::Event::MouseEnter, ADD_EVENT_BUTTON_EVENT)
        .on(ui::Event::MouseLeave, BUTTON_MOUSE_LEAVE)
        .radius(8)
        .padding(14)
        .margin_top(10)
        .bg(
            if state.hovered_button.as_deref() == Some(ADD_EVENT_BUTTON_EVENT) {
                "#4b4b4b"
            } else {
                "#2A2A2A"
            },
        )
        .width_full();

    // 组合添加事件界面
    container
        .child(
            ui::Element::new(ui::ElementType::Div, None)
                .child(event_name_label)
                .child(event_name_input),
        )
        .child(
            ui::Element::new(ui::ElementType::Div, None)
                .child(event_time_label)
                .child(event_time_input),
        )
        .child(
            on_index_container
                .child(on_index_label)
                .child(on_index_buttons.child(on_index_yes).child(on_index_no)),
        )
        .child(
            if_staring_day_container.child(if_staring_day_label).child(
                if_staring_day_buttons
                    .child(if_staring_day_yes)
                    .child(if_staring_day_no),
            ),
        )
        .child(add_event_button)
}

// 构建修改事件界面
fn build_modify_event_ui(state: &UiState) -> ui::Element {
    let container = ui::Element::new(ui::ElementType::Div, None)
        .flex()
        .flex_direction(ui::FlexDirection::Column);

    // 选择事件
    let select_event_label = ui::Element::new(ui::ElementType::P, Some("在这里选择你要修改的事件"))
        .size(16)
        .margin_bottom(8);
    let select_event_text = if state.selected_event_name.is_some() {
        state.selected_event_name.as_deref().unwrap_or("")
    } else {
        "选择事件"
    };
    let mut select_event_dropdown =
        ui::Element::new(ui::ElementType::Select, Some(select_event_text))
            .on(ui::Event::Change, SELECT_EVENT_DROPDOWN_EVENT)
            .radius(8)
            .padding(12)
            .bg("#2A2A2A")
            .width_full()
            .margin_bottom(8);

    // 动态添加事件选项
    if state.all_events.is_empty() {
        let option = ui::Element::new(ui::ElementType::Option, Some("选择事件"));
        select_event_dropdown = select_event_dropdown.child(option);
    } else {
        for (index, event_data) in state.all_events.iter().enumerate() {
            let option_text = format!("{}　{}", index + 1, event_data.name);
            let option = ui::Element::new(ui::ElementType::Option, Some(&option_text));
            select_event_dropdown = select_event_dropdown.child(option);
        }
    }

    // 事件名称输入
    let event_name_label = ui::Element::new(ui::ElementType::P, Some("输入事件名称"))
        .size(16)
        .margin_bottom(8);
    let event_name_input =
        ui::Element::new(ui::ElementType::Input, Some(&state.modify_event_data.name))
            .on(ui::Event::Change, MODIFY_EVENT_NAME_INPUT_EVENT)
            .radius(8)
            .bg("#2A2A2A")
            .width_full()
            .margin_bottom(8);

    // 事件时间输入
    let event_time_label = ui::Element::new(
        ui::ElementType::P,
        Some("输入事件目标日（格式必须为YYYY-MM-DD）"),
    )
    .size(16)
    .margin_bottom(8);
    let event_time_input =
        ui::Element::new(ui::ElementType::Input, Some(&state.modify_event_data.time))
            .on(ui::Event::Change, MODIFY_EVENT_TIME_INPUT_EVENT)
            .radius(8)
            .bg("#2A2A2A")
            .width_full()
            .margin_bottom(8);

    // 是否显示在主页
    let on_index_container = ui::Element::new(ui::ElementType::Div, None)
        .relative()
        .margin_bottom(8)
        .height(40)
        .width_full();
    let on_index_label = ui::Element::new(ui::ElementType::P, Some("是否显示在主页"))
        .absolute()
        .size(16)
        .left(0)
        .top(10);
    let on_index_buttons = ui::Element::new(ui::ElementType::Div, None)
        .absolute()
        .flex()
        .flex_direction(ui::FlexDirection::Row)
        .right(0)
        .top(5);
    let on_index_yes = ui::Element::new(ui::ElementType::Button, Some("是"))
        .without_default_styles()
        .on(ui::Event::Click, MODIFY_ON_INDEX_YES_EVENT)
        .on(ui::Event::MouseEnter, MODIFY_ON_INDEX_YES_EVENT)
        .on(ui::Event::MouseLeave, BUTTON_MOUSE_LEAVE)
        .radius(8)
        .padding_top(8)
        .padding_right(25)
        .padding_bottom(8)
        .padding_left(25)
        .bg(if state.modify_event_data.on_index {
            "#424242"
        } else if state.hovered_button.as_deref() == Some(MODIFY_ON_INDEX_YES_EVENT) {
            "#4b4b4b"
        } else {
            "#2A2A2A"
        })
        .margin_right(5);
    let on_index_no = ui::Element::new(ui::ElementType::Button, Some("否"))
        .without_default_styles()
        .on(ui::Event::Click, MODIFY_ON_INDEX_NO_EVENT)
        .on(ui::Event::MouseEnter, MODIFY_ON_INDEX_NO_EVENT)
        .on(ui::Event::MouseLeave, BUTTON_MOUSE_LEAVE)
        .radius(8)
        .padding_top(8)
        .padding_right(25)
        .padding_bottom(8)
        .padding_left(25)
        .bg(if !state.modify_event_data.on_index {
            "#424242"
        } else if state.hovered_button.as_deref() == Some(MODIFY_ON_INDEX_NO_EVENT) {
            "#4b4b4b"
        } else {
            "#2A2A2A"
        });

    // 是否计入起始日
    let if_staring_day_container = ui::Element::new(ui::ElementType::Div, None)
        .relative()
        .margin_bottom(8)
        .height(40)
        .width_full();
    let if_staring_day_label = ui::Element::new(ui::ElementType::P, Some("是否计入起始日"))
        .absolute()
        .size(16)
        .left(0)
        .top(10);
    let if_staring_day_buttons = ui::Element::new(ui::ElementType::Div, None)
        .absolute()
        .flex()
        .flex_direction(ui::FlexDirection::Row)
        .right(0)
        .top(5);
    let if_staring_day_yes = ui::Element::new(ui::ElementType::Button, Some("是"))
        .without_default_styles()
        .on(ui::Event::Click, MODIFY_IF_STARING_DAY_YES_EVENT)
        .on(ui::Event::MouseEnter, MODIFY_IF_STARING_DAY_YES_EVENT)
        .on(ui::Event::MouseLeave, BUTTON_MOUSE_LEAVE)
        .radius(8)
        .padding_top(8)
        .padding_right(25)
        .padding_bottom(8)
        .padding_left(25)
        .bg(if state.modify_event_data.if_staring_day {
            "#424242"
        } else if state.hovered_button.as_deref() == Some(MODIFY_IF_STARING_DAY_YES_EVENT) {
            "#4b4b4b"
        } else {
            "#2A2A2A"
        })
        .margin_right(5);
    let if_staring_day_no = ui::Element::new(ui::ElementType::Button, Some("否"))
        .without_default_styles()
        .on(ui::Event::Click, MODIFY_IF_STARING_DAY_NO_EVENT)
        .on(ui::Event::MouseEnter, MODIFY_IF_STARING_DAY_NO_EVENT)
        .on(ui::Event::MouseLeave, BUTTON_MOUSE_LEAVE)
        .radius(8)
        .padding_top(8)
        .padding_right(25)
        .padding_bottom(8)
        .padding_left(25)
        .bg(if !state.modify_event_data.if_staring_day {
            "#424242"
        } else if state.hovered_button.as_deref() == Some(MODIFY_IF_STARING_DAY_NO_EVENT) {
            "#4b4b4b"
        } else {
            "#2A2A2A"
        });

    // 获取事件按钮
    let get_events_button = ui::Element::new(ui::ElementType::Button, Some("获取手环端数据"))
        .without_default_styles()
        .on(ui::Event::Click, GET_EVENTS_BUTTON_EVENT)
        .on(ui::Event::MouseEnter, GET_EVENTS_BUTTON_EVENT)
        .on(ui::Event::MouseLeave, BUTTON_MOUSE_LEAVE)
        .radius(8)
        .padding(14)
        .bg(
            if state.hovered_button.as_deref() == Some(GET_EVENTS_BUTTON_EVENT) {
                "#4b4b4b"
            } else {
                "#2A2A2A"
            },
        )
        .width_full()
        .margin_bottom(8);

    // 同步按钮
    let sync_button = ui::Element::new(ui::ElementType::Button, Some("同步"))
        .without_default_styles()
        .on(ui::Event::Click, CHANGE_EVENT_BUTTON_EVENT)
        .on(ui::Event::MouseEnter, CHANGE_EVENT_BUTTON_EVENT)
        .on(ui::Event::MouseLeave, BUTTON_MOUSE_LEAVE)
        .radius(8)
        .padding(14)
        .bg(
            if state.hovered_button.as_deref() == Some(CHANGE_EVENT_BUTTON_EVENT) {
                "#4b4b4b"
            } else {
                "#2A2A2A"
            },
        )
        .width_full();

    // 根据状态决定显示哪个按钮
    let button_group = if state.has_fetched_events {
        ui::Element::new(ui::ElementType::Div, None)
            .flex()
            .flex_direction(ui::FlexDirection::Column)
            .child(sync_button)
    } else {
        ui::Element::new(ui::ElementType::Div, None)
            .flex()
            .flex_direction(ui::FlexDirection::Column)
            .child(get_events_button)
    };

    // 组合修改事件界面
    container
        .child(
            ui::Element::new(ui::ElementType::Div, None)
                .child(select_event_label)
                .child(select_event_dropdown),
        )
        .child(
            ui::Element::new(ui::ElementType::Div, None)
                .child(event_name_label)
                .child(event_name_input),
        )
        .child(
            ui::Element::new(ui::ElementType::Div, None)
                .child(event_time_label)
                .child(event_time_input),
        )
        .child(
            on_index_container
                .child(on_index_label)
                .child(on_index_buttons.child(on_index_yes).child(on_index_no)),
        )
        .child(
            if_staring_day_container.child(if_staring_day_label).child(
                if_staring_day_buttons
                    .child(if_staring_day_yes)
                    .child(if_staring_day_no),
            ),
        )
        .child(button_group)
}

// 构建删除事件界面
fn build_delete_event_ui(state: &UiState) -> ui::Element {
    let container = ui::Element::new(ui::ElementType::Div, None)
        .flex()
        .flex_direction(ui::FlexDirection::Column);

    // 选择事件
    let select_event_label = ui::Element::new(ui::ElementType::P, Some("在这里选择你要删除的事件"))
        .size(16)
        .margin_bottom(8);
    let select_event_text = if state.selected_event_name.is_some() {
        state.selected_event_name.as_deref().unwrap_or("")
    } else {
        "选择事件"
    };
    let mut select_event_dropdown = ui::Element::new(ui::ElementType::Select, Some(select_event_text))
        .on(ui::Event::Change, SELECT_EVENT_DROPDOWN_EVENT)
        .radius(8)
        .padding(12)
        .bg("#2A2A2A")
        .width_full()
        .margin_bottom(8);

    // 动态添加事件选项
    if state.all_events.is_empty() {
        let option = ui::Element::new(ui::ElementType::Option, Some("选择事件"));
        select_event_dropdown = select_event_dropdown.child(option);
    } else {
        for (index, event_data) in state.all_events.iter().enumerate() {
            let option_text = format!("{}　{}", index + 1, event_data.name);
            let option = ui::Element::new(ui::ElementType::Option, Some(&option_text));
            select_event_dropdown = select_event_dropdown.child(option);
        }
    }

    // 获取事件按钮
    let get_events_button = ui::Element::new(ui::ElementType::Button, Some("获取手环端数据"))
        .without_default_styles()
        .on(ui::Event::Click, GET_EVENTS_BUTTON_EVENT)
        .on(ui::Event::MouseEnter, GET_EVENTS_BUTTON_EVENT)
        .on(ui::Event::MouseLeave, BUTTON_MOUSE_LEAVE)
        .radius(8)
        .padding(14)
        .bg(
            if state.hovered_button.as_deref() == Some(GET_EVENTS_BUTTON_EVENT) {
                "#4b4b4b"
            } else {
                "#2A2A2A"
            },
        )
        .width_full()
        .margin_bottom(8);

    // 删除按钮
    let delete_button = ui::Element::new(ui::ElementType::Button, Some("删除"))
        .without_default_styles()
        .on(ui::Event::Click, DELETE_EVENT_BUTTON_EVENT)
        .on(ui::Event::MouseEnter, DELETE_EVENT_BUTTON_EVENT)
        .on(ui::Event::MouseLeave, BUTTON_MOUSE_LEAVE)
        .radius(8)
        .padding(14)
        .bg(
            if state.hovered_button.as_deref() == Some(DELETE_EVENT_BUTTON_EVENT) {
                "#4b4b4b"
            } else {
                "#2A2A2A"
            },
        )
        .width_full();

    // 根据状态决定显示哪个按钮
    let button_group = if state.has_fetched_events {
        ui::Element::new(ui::ElementType::Div, None)
            .flex()
            .flex_direction(ui::FlexDirection::Column)
            .child(delete_button)
    } else {
        ui::Element::new(ui::ElementType::Div, None)
            .flex()
            .flex_direction(ui::FlexDirection::Column)
            .child(get_events_button)
    };

    // 组合删除事件界面
    container
        .child(
            ui::Element::new(ui::ElementType::Div, None)
                .child(select_event_label)
                .child(select_event_dropdown),
        )
        .child(button_group)
}

// 构建主界面
pub fn build_main_ui() -> ui::Element {
    let state = ui_state()
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    // 主容器
    let main_container = ui::Element::new(ui::ElementType::Div, None)
        .flex()
        .flex_direction(ui::FlexDirection::Column)
        .width_full()
        .padding(20);

    // 错误提示元素
    let error_element = if let Some(ref msg) = state.error_message {
        let bg_color = if state.is_success_message {
            "#4CAF50" // 绿色用于成功消息
        } else {
            "#FF4444" // 红色用于错误消息
        };
        Some(
            ui::Element::new(ui::ElementType::Div, None)
                .bg(bg_color)
                .radius(8)
                .padding(12)
                .margin_bottom(20)
                .on(ui::Event::Click, HIDE_ERROR_EVENT)
                .child(
                    ui::Element::new(ui::ElementType::P, Some(msg))
                        .size(14)
                        .text_color("#FFFFFF"),
                ),
        )
    } else {
        None
    };

    // 为了让标签页容器居中，我们可以在它外面再包裹一层容器
    let tabs_wrapper = ui::Element::new(ui::ElementType::Div, None)
        .flex()
        .justify_center();

    // 标签页容器
    let tab_container = ui::Element::new(ui::ElementType::Div, None)
        .flex()
        .flex_direction(ui::FlexDirection::Row)
        .margin_bottom(20)
        .bg("#1E1E1F")
        .radius(8);

    // 标签页按钮
    let add_event_tab = ui::Element::new(ui::ElementType::Button, Some("添加事件"))
        .without_default_styles()
        .padding_left(20)
        .padding_right(20)
        .padding_top(12)
        .padding_bottom(12)
        .margin(5)
        .bg(if state.current_tab == EventType::AddEvent {
            "#424242"
        } else if state.hovered_button.as_deref() == Some(TAB_ADD_EVENT) {
            "#4b4b4b"
        } else {
            "#2A2A2A"
        })
        .on(ui::Event::Click, TAB_ADD_EVENT)
        .on(ui::Event::MouseEnter, TAB_ADD_EVENT)
        .on(ui::Event::MouseLeave, BUTTON_MOUSE_LEAVE)
        .radius(8);

    let modify_event_tab = ui::Element::new(ui::ElementType::Button, Some("修改事件"))
        .without_default_styles()
        .padding_left(20)
        .padding_right(20)
        .padding_top(12)
        .padding_bottom(12)
        .margin(5)
        .bg(if state.current_tab == EventType::ModifyEvent {
            "#424242"
        } else if state.hovered_button.as_deref() == Some(TAB_MODIFY_EVENT) {
            "#4b4b4b"
        } else {
            "#2A2A2A"
        })
        .on(ui::Event::Click, TAB_MODIFY_EVENT)
        .on(ui::Event::MouseEnter, TAB_MODIFY_EVENT)
        .on(ui::Event::MouseLeave, BUTTON_MOUSE_LEAVE)
        .radius(8);

    let delete_event_tab = ui::Element::new(ui::ElementType::Button, Some("删除事件"))
        .without_default_styles()
        .padding_left(20)
        .padding_right(20)
        .padding_top(12)
        .padding_bottom(12)
        .margin(5)
        .bg(if state.current_tab == EventType::DeleteEvent {
            "#424242"
        } else if state.hovered_button.as_deref() == Some(TAB_DELETE_EVENT) {
            "#4b4b4b"
        } else {
            "#2A2A2A"
        })
        .on(ui::Event::Click, TAB_DELETE_EVENT)
        .on(ui::Event::MouseEnter, TAB_DELETE_EVENT)
        .on(ui::Event::MouseLeave, BUTTON_MOUSE_LEAVE)
        .radius(8);

    let tabs = tab_container
        .child(add_event_tab)
        .child(modify_event_tab)
        .child(delete_event_tab);

    // 内容容器
    let content_container = ui::Element::new(ui::ElementType::Div, None)
        .flex()
        .flex_direction(ui::FlexDirection::Column)
        .padding_top(20)
        .padding_bottom(20);

    // 根据当前标签页构建对应界面
    let content = match state.current_tab {
        EventType::AddEvent => build_add_event_ui(&state),
        EventType::ModifyEvent => build_modify_event_ui(&state),
        EventType::DeleteEvent => build_delete_event_ui(&state),
    };

    // 将标签页添加到包装器中
    let tabs_wrapper = tabs_wrapper.child(tabs);

    // 组合主界面
    let mut main_container = main_container;

    if let Some(error) = error_element {
        main_container = main_container.child(error);
    }

    main_container
        .child(tabs_wrapper)
        .child(content_container.child(content))
}

// 渲染主界面
pub fn render_main_ui(element_id: &str) {
    let mut state = ui_state()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    state.root_element_id = Some(element_id.to_string());

    // 释放写锁，避免在build_main_ui中尝试获取读锁时产生死锁
    drop(state);

    psys_host::ui::render(element_id, build_main_ui());
}
