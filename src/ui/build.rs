use crate::astrobox::psys_host::{self, ui};
use super::state::{ui_state, EventType, UiState};
use super::event_handler::{EVENT_NAME_INPUT_EVENT, EVENT_TIME_INPUT_EVENT, ON_INDEX_YES_EVENT, ON_INDEX_NO_EVENT, IF_STARING_DAY_YES_EVENT, IF_STARING_DAY_NO_EVENT, ADD_EVENT_BUTTON_EVENT, BUTTON_MOUSE_LEAVE, MODIFY_EVENT_NAME_INPUT_EVENT, MODIFY_EVENT_TIME_INPUT_EVENT, MODIFY_ON_INDEX_YES_EVENT, MODIFY_ON_INDEX_NO_EVENT, MODIFY_IF_STARING_DAY_YES_EVENT, MODIFY_IF_STARING_DAY_NO_EVENT, GET_EVENTS_BUTTON_EVENT, CHANGE_EVENT_BUTTON_EVENT, DELETE_EVENT_BUTTON_EVENT, SELECT_EVENT_DROPDOWN_EVENT, TAB_ADD_EVENT, TAB_MODIFY_EVENT, TAB_DELETE_EVENT, HIDE_ERROR_EVENT};

pub fn build_add_event_ui(state: &UiState) -> ui::Element {
    let container = ui::Element::new(ui::ElementType::Div, None)
        .flex()
        .flex_direction(ui::FlexDirection::Column);

    let event_name_label = ui::Element::new(ui::ElementType::P, Some("输入事件名称"))
        .size(16)
        .margin_bottom(8);
    let event_name_input = ui::Element::new(ui::ElementType::Input, Some(&state.event_data.name))
        .on(ui::Event::Change, EVENT_NAME_INPUT_EVENT)
        .radius(8)
        .bg("#2A2A2A")
        .width_full()
        .margin_bottom(8);

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

pub fn build_modify_event_ui(state: &UiState) -> ui::Element {
    let container = ui::Element::new(ui::ElementType::Div, None)
        .flex()
        .flex_direction(ui::FlexDirection::Column);

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

pub fn build_delete_event_ui(state: &UiState) -> ui::Element {
    let container = ui::Element::new(ui::ElementType::Div, None)
        .flex()
        .flex_direction(ui::FlexDirection::Column);

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

    container
        .child(
            ui::Element::new(ui::ElementType::Div, None)
                .child(select_event_label)
                .child(select_event_dropdown),
        )
        .child(button_group)
}

pub fn build_main_ui() -> ui::Element {
    let state = ui_state()
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let main_container = ui::Element::new(ui::ElementType::Div, None)
        .flex()
        .flex_direction(ui::FlexDirection::Column)
        .width_full()
        .padding(20);

    let error_element = if let Some(ref msg) = state.error_message {
        let bg_color = if state.is_success_message {
            "#4CAF50"
        } else {
            "#FF4444"
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

    let tabs_wrapper = ui::Element::new(ui::ElementType::Div, None)
        .flex()
        .justify_center();

    let tab_container = ui::Element::new(ui::ElementType::Div, None)
        .flex()
        .flex_direction(ui::FlexDirection::Row)
        .margin_bottom(20)
        .bg("#1E1E1F")
        .radius(8);

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

    let content_container = ui::Element::new(ui::ElementType::Div, None)
        .flex()
        .flex_direction(ui::FlexDirection::Column)
        .padding_top(20)
        .padding_bottom(20);

    let content = match state.current_tab {
        EventType::AddEvent => build_add_event_ui(&state),
        EventType::ModifyEvent => build_modify_event_ui(&state),
        EventType::DeleteEvent => build_delete_event_ui(&state),
    };

    let tabs_wrapper = tabs_wrapper.child(tabs);

    let mut main_container = main_container;

    if let Some(error) = error_element {
        main_container = main_container.child(error);
    }

    main_container
        .child(tabs_wrapper)
        .child(content_container.child(content))
}

pub fn render_main_ui(element_id: &str) {
    let mut state = ui_state()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    state.root_element_id = Some(element_id.to_string());
    drop(state);
    psys_host::ui::render(element_id, build_main_ui());
}
