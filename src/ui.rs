use crate::astrobox::psys_host::{self, ui};
use std::sync::{OnceLock, RwLock};

// 事件类型枚举
#[derive(Clone, Copy, PartialEq)]
pub enum EventType {
    AddEvent,
    ModifyEvent,
    DeleteEvent,
}

// 事件数据结构
#[derive(Default, Debug)]
struct EventData {
    name: String,
    time: String,
    on_index: bool,
    if_staring_day: bool,
}

// UI状态管理
struct UiState {
    root_element_id: Option<String>,
    current_tab: EventType,
    event_data: EventData,
    modify_event_data: EventData,
    all_events: Vec<(String, String)>, // (event_name, event_date)
    selected_event_index: Option<usize>,
    hovered_button: Option<String>, // 跟踪当前悬停的按钮
}

static UI_STATE: OnceLock<RwLock<UiState>> = OnceLock::new();

fn ui_state() -> &'static RwLock<UiState> {
    UI_STATE.get_or_init(|| {
        RwLock::new(UiState {
            root_element_id: None,
            current_tab: EventType::AddEvent,
            event_data: EventData::default(),
            modify_event_data: EventData::default(),
            all_events: Vec::new(),
            selected_event_index: None,
            hovered_button: None,
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

// 标签页切换处理
fn handle_tab_change(_event: &str, value: &str) {
    let mut state = ui_state()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    state.current_tab = match value {
        "添加事件" => EventType::AddEvent,
        "修改事件" => EventType::ModifyEvent,
        "删除事件" => EventType::DeleteEvent,
        _ => EventType::AddEvent,
    };
}

// 输入事件处理
fn handle_input_event(event: &str, value: &str) {
    let mut state = ui_state()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    match event {
        EVENT_NAME_INPUT_EVENT => {
            state.event_data.name = value.to_string();
        }
        EVENT_TIME_INPUT_EVENT => {
            state.event_data.time = value.to_string();
        }
        MODIFY_EVENT_NAME_INPUT_EVENT => {
            state.modify_event_data.name = value.to_string();
        }
        MODIFY_EVENT_TIME_INPUT_EVENT => {
            state.modify_event_data.time = value.to_string();
        }
        _ => {
            tracing::info!("未处理的事件类型");
        }
    }
}

// 下拉菜单事件处理
fn handle_dropdown_event(event: &str, value: &str) {
    let mut state = ui_state()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    match event {
        ON_INDEX_CHANGE_EVENT => {
            state.event_data.on_index = value == "是";
        }
        IF_STARING_DAY_CHANGE_EVENT => {
            state.event_data.if_staring_day = value == "是";
        }
        MODIFY_ON_INDEX_CHANGE_EVENT => {
            state.modify_event_data.on_index = value == "是";
        }
        MODIFY_IF_STARING_DAY_CHANGE_EVENT => {
            state.modify_event_data.if_staring_day = value == "是";
        }
        SELECT_EVENT_DROPDOWN_EVENT => {
            // 解析选择的事件索引
            if let Some(space_idx) = value.find("　") {
                if let Ok(index) = value[..space_idx].parse::<usize>() {
                    state.selected_event_index = Some(index - 1);
                    // 加载选中事件的数据
                    let event_data = state.all_events.get(index - 1).cloned();
                    if let Some((name, time)) = event_data {
                        state.modify_event_data.name = name;
                        state.modify_event_data.time = time;
                    }
                }
            }
        }
        _ => {}
    }
}

// 按钮点击事件处理
fn handle_button_click(event: &str) {
    match event {
        ADD_EVENT_BUTTON_EVENT => {
            // 添加事件逻辑
            tracing::info!(
                "添加事件: {:?}",
                ui_state()
                    .read()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .event_data
            );
        }
        GET_EVENTS_BUTTON_EVENT => {
            // 获取事件列表逻辑
            tracing::info!("获取手环端数据");
        }
        CHANGE_EVENT_BUTTON_EVENT => {
            // 修改事件逻辑
            tracing::info!(
                "修改事件: {:?}",
                ui_state()
                    .read()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .modify_event_data
            );
        }
        DELETE_EVENT_BUTTON_EVENT => {
            // 删除事件逻辑
            tracing::info!(
                "删除事件: {:?}",
                ui_state()
                    .read()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .selected_event_index
            );
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

// 事件处理器
pub fn ui_event_processor(evtype: ui::Event, event: &str) {
    // 输出事件类型的原始字符串表示
    let evtype_str = format!("{:?}", evtype);
    tracing::info!("接收到事件: 类型={}, 名称={}", evtype_str, event);
    match evtype {
        ui::Event::Click => {
            handle_button_click(event);
        }
        ui::Event::Change => {
            // 处理输入和下拉菜单的变化
            // 注意：这里需要获取实际的输入值，当前接口可能无法直接获取
            // 实际实现中可能需要通过其他方式获取输入值
            tracing::info!("Change event: {}", event);
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
        .padding(12)
        .bg("#2A2A2D")
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
        .padding(12)
        .bg("#2A2A2D")
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
        .on(ui::Event::Click, ON_INDEX_YES_EVENT)
        .radius(8)
        .padding_top(8)
        .padding_right(25)
        .padding_bottom(8)
        .padding_left(25)
        .bg(if state.event_data.on_index {
            "#424243"
        } else {
            "#2A2A2D"
        })
        .margin_right(5);
    let on_index_no = ui::Element::new(ui::ElementType::Button, Some("否"))
        .on(ui::Event::Click, ON_INDEX_NO_EVENT)
        .radius(8)
        .padding_top(8)
        .padding_right(25)
        .padding_bottom(8)
        .padding_left(25)
        .bg(if !state.event_data.on_index {
            "#424243"
        } else {
            "#2A2A2D"
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
        .on(ui::Event::Click, IF_STARING_DAY_YES_EVENT)
        .radius(8)
        .padding_top(8)
        .padding_right(25)
        .padding_bottom(8)
        .padding_left(25)
        .bg(if state.event_data.if_staring_day {
            "#424243"
        } else {
            "#2A2A2D"
        })
        .margin_right(5);
    let if_staring_day_no = ui::Element::new(ui::ElementType::Button, Some("否"))
        .on(ui::Event::Click, IF_STARING_DAY_NO_EVENT)
        .radius(8)
        .padding_top(8)
        .padding_right(25)
        .padding_bottom(8)
        .padding_left(25)
        .bg(if !state.event_data.if_staring_day {
            "#424243"
        } else {
            "#2A2A2D"
        });

    // 添加事件按钮
    let add_event_button = ui::Element::new(ui::ElementType::Button, Some("发送"))
        .on(ui::Event::Click, ADD_EVENT_BUTTON_EVENT)
        .radius(8)
        .padding(14)
        .margin_top(10)
        .bg("#2A2A2D")
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
    let select_event_dropdown = ui::Element::new(ui::ElementType::Select, Some("选择事件"))
        .on(ui::Event::Change, SELECT_EVENT_DROPDOWN_EVENT)
        .radius(8)
        .padding(12)
        .bg("#2A2A2D")
        .width_full()
        .margin_bottom(8);

    // 事件名称输入
    let event_name_label = ui::Element::new(ui::ElementType::P, Some("输入事件名称"))
        .size(16)
        .margin_bottom(8);
    let event_name_input =
        ui::Element::new(ui::ElementType::Input, Some(&state.modify_event_data.name))
            .on(ui::Event::Change, MODIFY_EVENT_NAME_INPUT_EVENT)
            .radius(8)
            .padding(12)
            .bg("#2A2A2D")
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
            .padding(12)
            .bg("#2A2A2D")
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
        .on(ui::Event::Click, MODIFY_ON_INDEX_YES_EVENT)
        .radius(8)
        .padding_top(8)
        .padding_right(25)
        .padding_bottom(8)
        .padding_left(25)
        .bg(if state.modify_event_data.on_index {
            "#424243"
        } else {
            "#2A2A2D"
        })
        .margin_right(5);
    let on_index_no = ui::Element::new(ui::ElementType::Button, Some("否"))
        .on(ui::Event::Click, MODIFY_ON_INDEX_NO_EVENT)
        .radius(8)
        .padding_top(8)
        .padding_right(25)
        .padding_bottom(8)
        .padding_left(25)
        .bg(if !state.modify_event_data.on_index {
            "#424243"
        } else {
            "#2A2A2D"
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
        .on(ui::Event::Click, MODIFY_IF_STARING_DAY_YES_EVENT)
        .radius(8)
        .padding_top(8)
        .padding_right(25)
        .padding_bottom(8)
        .padding_left(25)
        .bg(if state.modify_event_data.if_staring_day {
            "#424243"
        } else {
            "#2A2A2D"
        })
        .margin_right(5);
    let if_staring_day_no = ui::Element::new(ui::ElementType::Button, Some("否"))
        .on(ui::Event::Click, MODIFY_IF_STARING_DAY_NO_EVENT)
        .radius(8)
        .padding_top(8)
        .padding_right(25)
        .padding_bottom(8)
        .padding_left(25)
        .bg(if !state.modify_event_data.if_staring_day {
            "#424243"
        } else {
            "#2A2A2D"
        });

    // 按钮组容器
    let button_group = ui::Element::new(ui::ElementType::Div, None)
        .flex()
        .flex_direction(ui::FlexDirection::Column);

    // 获取事件按钮
    let get_events_button = ui::Element::new(ui::ElementType::Button, Some("获取手环端数据"))
        .on(ui::Event::Click, GET_EVENTS_BUTTON_EVENT)
        .radius(8)
        .padding(12)
        .bg("#2A2A2D")
        .width_full()
        .margin_bottom(8);

    // 同步按钮
    let sync_button = ui::Element::new(ui::ElementType::Button, Some("同步"))
        .on(ui::Event::Click, CHANGE_EVENT_BUTTON_EVENT)
        .radius(8)
        .padding(14)
        .bg("#2A2A2D")
        .width_full();

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
        .child(button_group.child(get_events_button).child(sync_button))
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
    let select_event_dropdown = ui::Element::new(ui::ElementType::Select, Some("选择事件"))
        .on(ui::Event::Change, SELECT_EVENT_DROPDOWN_EVENT)
        .radius(8)
        .padding(12)
        .bg("#2A2A2D")
        .width_full()
        .margin_bottom(8);

    // 按钮组容器
    let button_group = ui::Element::new(ui::ElementType::Div, None)
        .flex()
        .flex_direction(ui::FlexDirection::Column);

    // 获取事件按钮
    let get_events_button = ui::Element::new(ui::ElementType::Button, Some("获取手环端数据"))
        .on(ui::Event::Click, GET_EVENTS_BUTTON_EVENT)
        .radius(8)
        .padding(12)
        .bg("#2A2A2D")
        .width_full()
        .margin_bottom(8);

    // 删除按钮
    let delete_button = ui::Element::new(ui::ElementType::Button, Some("删除"))
        .on(ui::Event::Click, DELETE_EVENT_BUTTON_EVENT)
        .radius(8)
        .padding(14)
        .bg("#2A2A2D")
        .width_full();

    // 组合删除事件界面
    container
        .child(
            ui::Element::new(ui::ElementType::Div, None)
                .child(select_event_label)
                .child(select_event_dropdown),
        )
        .child(button_group.child(get_events_button).child(delete_button))
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
        .padding_left(20)
        .padding_right(20)
        .padding_top(12)
        .padding_bottom(12)
        .margin(5)
        .bg(if state.current_tab == EventType::AddEvent {
            "#424243"
        } else {
            "#2A2A2D"
        })
        .on(ui::Event::Click, TAB_ADD_EVENT)
        .radius(8);

    let modify_event_tab = ui::Element::new(ui::ElementType::Button, Some("修改事件"))
        .padding_left(20)
        .padding_right(20)
        .padding_top(12)
        .padding_bottom(12)
        .margin(5)
        .bg(if state.current_tab == EventType::ModifyEvent {
            "#424243"
        } else {
            "#2A2A2D"
        })
        .on(ui::Event::Click, TAB_MODIFY_EVENT)
        .radius(8);

    let delete_event_tab = ui::Element::new(ui::ElementType::Button, Some("删除事件"))
        .padding_left(20)
        .padding_right(20)
        .padding_top(12)
        .padding_bottom(12)
        .margin(5)
        .bg(if state.current_tab == EventType::DeleteEvent {
            "#424243"
        } else {
            "#2A2A2D"
        })
        .on(ui::Event::Click, TAB_DELETE_EVENT)
        .radius(8);

    let tabs = tab_container
        .child(add_event_tab)
        .child(modify_event_tab)
        .child(delete_event_tab);

    // 内容容器
    let content_container = ui::Element::new(ui::ElementType::Div, None)
        .flex()
        .flex_direction(ui::FlexDirection::Column)
        .margin_left(10)
        .margin_right(10)
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
