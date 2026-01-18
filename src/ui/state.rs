use std::sync::{OnceLock, RwLock};
use chrono::{Datelike, Utc};

#[derive(Debug, Clone, PartialEq)]
pub enum EventType {
    AddEvent,
    ModifyEvent,
    DeleteEvent,
}

#[derive(Debug, Clone)]
pub struct EventData {
    pub name: String,
    pub time: String,
    pub on_index: bool,
    pub if_staring_day: bool,
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

pub struct UiState {
    pub root_element_id: Option<String>,
    pub current_tab: EventType,
    pub event_data: EventData,
    pub modify_event_data: EventData,
    pub all_events: Vec<EventData>,
    pub selected_event_index: Option<usize>,
    pub selected_event_name: Option<String>,
    pub has_fetched_events: bool,
    pub hovered_button: Option<String>,
    pub error_message: Option<String>,
    pub is_success_message: bool,
    pub message_timer_id: Option<u64>,
}

static UI_STATE: OnceLock<RwLock<UiState>> = OnceLock::new();

pub fn ui_state() -> &'static RwLock<UiState> {
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
            message_timer_id: None,
        })
    })
}

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
pub const HIDE_ERROR_EVENT: &str = "hide_error";
pub const BUTTON_MOUSE_LEAVE: &str = "button_mouse_leave";
pub const TAB_ADD_EVENT: &str = "tab_add_event";
pub const TAB_MODIFY_EVENT: &str = "tab_modify_event";
pub const TAB_DELETE_EVENT: &str = "tab_delete_event";

pub fn get_current_date() -> String {
    let utc_now = Utc::now();
    
    let timezone_offset_minutes = wit_bindgen::block_on(async {
        crate::astrobox::psys_host::os::timezone_offset_minutes().await
    });
    
    let local_now = utc_now.with_timezone(&chrono::FixedOffset::east_opt(timezone_offset_minutes * 60).unwrap());
    tracing::info!("local_now: {:?}", local_now);
    format!("{:04}-{:02}-{:02}", local_now.year(), local_now.month(), local_now.day())
}
 