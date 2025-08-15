import AstroBox from "astrobox-plugin-sdk";
import dayjs from "dayjs";
import customParseFormat from "dayjs/plugin/customParseFormat";

let ui;
let getAllEvent = [];
let index;
let whoGetEvent = 0;

const event = {
  name: "",
  time: dayjs().format("YYYY-MM-DD"),
  on_index: true,
  IFStaringDay: false,
};

const event1 = {
  name: "",
  time: "",
  on_index: true,
  IFStaringDay: false,
};

dayjs.extend(customParseFormat);

// UI服务启动
let onEventName = AstroBox.native.regNativeFun(eventName);
let onEventTime = AstroBox.native.regNativeFun(eventTime);
let onEventIndex = AstroBox.native.regNativeFun(eventIndex);
let onEventIFStaringDay = AstroBox.native.regNativeFun(eventIFStaringDay);
let onEventName1 = AstroBox.native.regNativeFun(eventName1);
let onEventTime1 = AstroBox.native.regNativeFun(eventTime1);
let onEventIndex1 = AstroBox.native.regNativeFun(eventIndex1);
let onEventIFStaringDay1 = AstroBox.native.regNativeFun(eventIFStaringDay1);
let AddEventId = AstroBox.native.regNativeFun(AddEvent);
let onTab = AstroBox.native.regNativeFun(Tab);
let getEventId = AstroBox.native.regNativeFun(getEvent);
let selectEventId = AstroBox.native.regNativeFun(selectEvent);
let ChangeEventId = AstroBox.native.regNativeFun(ChangeEvent);
let DeleteEventId = AstroBox.native.regNativeFun(DeleteEvent);

async function selectEvent(text) {
  let space = text.indexOf("　");
  index = parseInt(text.slice(0, space)) - 1;
  if (whoGetEvent) {
    return;
  }
  event1.name = getAllEvent[index].name;
  event1.time = getAllEvent[index].date;
  event1.on_index = getAllEvent[index].on_index;
  event1.IFStaringDay = getAllEvent[index].IFStaringDay;
  console.log(JSON.stringify(event1));
  ui[16] = {
    node_id: "eventName2",
    visibility: true,
    disabled: false,
    content: {
      type: "Input",
      value: { text: event1.name, callback_fun_id: onEventName1 },
    },
  };
  ui[18] = {
    node_id: "eventTime2",
    visibility: true,
    disabled: false,
    content: {
      type: "Input",
      value: { text: event1.time, callback_fun_id: onEventTime1 },
    },
  };
  ui[19].content.value = `<span style="margin-left: 13%;">是否显示在主页<span style="color:red;">（当前选中：${event1.on_index ? "是" : "否"}）</span></span>`;
  ui[21].content.value = `<span style="margin-left: 13%;">是否计入起始日<span style="color:red;">（当前选中：${event1.IFStaringDay ? "是" : "否"}）</span></span>`;
  ui[16].visibility = false;
  ui[18].visibility = false;
  AstroBox.ui.updatePluginSettingsUI(ui);
  await sleep(100);
  ui[16].visibility = true;
  ui[18].visibility = true;
  AstroBox.ui.updatePluginSettingsUI(ui);
}

async function Tab(text) {
  ui[2].content.value = `
          <span style="display:inline-block;width:100%;margin:10px 0;text-align:center;font-size:24px;font-weight:bold;">${text}</span>
          `;
  for (let i = 3; i <= ui.length - 1; i++) {
    ui[i].visibility = false;
  }
  switch (text) {
    case "添加事件":
      for (let i = 2; i <= 12; i++) {
        if (i != 11) {
          ui[i].visibility = true;
        }
      }
      break;
    case "修改事件":
      for (let i = 13; i <= 24; i++) {
        if (i != 23) {
          ui[i].visibility = true;
        }
      }
      whoGetEvent = 0;
      getAllEvent = [];
      event1.name = "";
      event1.time = "";
      event1.on_index = true;
      event1.IFStaringDay = false;
      index = "";
      ui[14].content.value.options = [];
      ui[16] = {
        node_id: "eventName2",
        visibility: true,
        disabled: false,
        content: {
          type: "Input",
          value: { text: event1.name, callback_fun_id: onEventName1 },
        },
      };
      ui[18] = {
        node_id: "eventTime2",
        visibility: true,
        disabled: false,
        content: {
          type: "Input",
          value: { text: event1.time, callback_fun_id: onEventTime1 },
        },
      };
      break;
    case "删除事件":
      for (let i = 26; i <= 29; i++) {
        if (i != 28) {
          ui[i].visibility = true;
        }
      }
      whoGetEvent = 1;
      getAllEvent = [];
      index = "";
      break;
    default:
      break;
  }
  AstroBox.ui.updatePluginSettingsUI(ui);
}

async function eventName(text) {
  event.name = text;
  ui[4] = {
    node_id: "eventName1",
    visibility: true,
    disabled: false,
    content: {
      type: "Input",
      value: { text: event.name, callback_fun_id: onEventName },
    },
  };
  AstroBox.ui.updatePluginSettingsUI(ui);
}

async function eventTime(text) {
  event.time = text;
  ui[6] = {
    node_id: "eventTime1",
    visibility: true,
    disabled: false,
    content: {
      type: "Input",
      value: { text: event.time, callback_fun_id: onEventTime },
    },
  };
  AstroBox.ui.updatePluginSettingsUI(ui);
}

async function eventIndex(text) {
  if (text == "否") {
    event.on_index = false;
    ui[7].content.value = `<span style="margin-left: 13%;">是否显示在主页<span style="color:red;">（当前选中：否）</span></span>`;
  } else {
    event.on_index = true;
    ui[7].content.value = `<span style="margin-left: 13%;">是否显示在主页<span style="color:red;">（当前选中：是）</span></span>`;
  }
  AstroBox.ui.updatePluginSettingsUI(ui);
}

async function eventIFStaringDay(text) {
  if (text == "是") {
    event.IFStaringDay = true;
    ui[9].content.value = `<span style="margin-left: 13%;">是否计入起始日<span style="color:red;">（当前选中：是）</span></span>`;
  } else {
    event.IFStaringDay = false;
    ui[9].content.value = `<span style="margin-left: 13%;">是否计入起始日<span style="color:red;">（当前选中：否）</span></span>`;
  }
  AstroBox.ui.updatePluginSettingsUI(ui);
}

async function eventIndex1(text) {
  if (text == "否") {
    event1.on_index = false;
    ui[19].content.value = `<span style="margin-left: 13%;">是否显示在主页<span style="color:red;">（当前选中：否）</span></span>`;
  } else {
    event1.on_index = true;
    ui[19].content.value = `<span style="margin-left: 13%;">是否显示在主页<span style="color:red;">（当前选中：是）</span></span>`;
  }
  AstroBox.ui.updatePluginSettingsUI(ui);
}

async function eventIFStaringDay1(text) {
  if (text == "是") {
    event1.IFStaringDay = true;
    ui[21].content.value = `<span style="margin-left: 13%;">是否计入起始日<span style="color:red;">（当前选中：是）</span></span>`;
  } else {
    event1.IFStaringDay = false;
    ui[21].content.value = `<span style="margin-left: 13%;">是否计入起始日<span style="color:red;">（当前选中：否）</span></span>`;
  }
  AstroBox.ui.updatePluginSettingsUI(ui);
}

async function eventName1(text) {
  event1.name = text;
}

async function eventTime1(text) {
  event1.time = text;
}

const sleep = (delay) => new Promise((resolve) => setTimeout(resolve, delay));

async function warning(warn, type = 11) {
  ui[type].content.value = `
          <span style="display:inline-block;width:100%;text-align:center;font-weight:bold;color:red;">${warn}</span>
          `;
  ui[type].visibility = true;
  AstroBox.ui.updatePluginSettingsUI(ui);
  await sleep(2000);
  ui[type].visibility = false;
  AstroBox.ui.updatePluginSettingsUI(ui);
}

AstroBox.lifecycle.onLoad(() => {
  ui = [
    {
      node_id: "html0",
      visibility: true,
      disabled: false,
      content: {
        type: "HtmlDocument",
        value: `
          <span style="margin-left: 13%;">在这里选择你要设置的类型</span>
          `,
      },
    },
    {
      node_id: "tab",
      visibility: true,
      disabled: false,
      content: {
        type: "Dropdown",
        value: {
          options: ["添加事件", "修改事件", "删除事件"],
          callback_fun_id: onTab,
        },
      },
    },
    // 添加事件UI（2～12），警告11
    {
      node_id: "title",
      visibility: true,
      disabled: false,
      content: {
        type: "HtmlDocument",
        value: `
          <span style="display:inline-block;width:100%;margin:10px 0;text-align:center;font-size:24px;font-weight:bold;">添加事件</span>
          `,
      },
    },
    {
      node_id: "html1",
      visibility: true,
      disabled: false,
      content: {
        type: "HtmlDocument",
        value: `
          <span style="margin-left: 13%;">输入事件名称</span>
          `,
      },
    },
    {
      node_id: "eventName1",
      visibility: true,
      disabled: false,
      content: {
        type: "Input",
        value: { text: event.name, callback_fun_id: onEventName },
      },
    },
    {
      node_id: "html2",
      visibility: true,
      disabled: false,
      content: {
        type: "HtmlDocument",
        value: `
          <span style="margin-left: 13%;">输入事件目标日<span style="color:red;">（格式必须为YYYY-MM-DD）</span></span>
          `,
      },
    },
    {
      node_id: "eventTime1",
      visibility: true,
      disabled: false,
      content: {
        type: "Input",
        value: { text: event.time, callback_fun_id: onEventTime },
      },
    },
    {
      node_id: "html31",
      visibility: true,
      disabled: false,
      content: {
        type: "HtmlDocument",
        value: `
          <span style="margin-left: 13%;">是否显示在主页<span style="color:red;">（当前选中：是）</span></span>
          `,
      },
    },
    {
      node_id: "tab11",
      visibility: true,
      disabled: false,
      content: {
        type: "Dropdown",
        value: {
          options: ["是", "否"],
          callback_fun_id: onEventIndex,
        },
      },
    },
    {
      node_id: "html32",
      visibility: true,
      disabled: false,
      content: {
        type: "HtmlDocument",
        value: `
          <span style="margin-left: 13%;">是否计入起始日<span style="color:red;">（当前选中：否）</span></span>
          `,
      },
    },
    {
      node_id: "tab12",
      visibility: true,
      disabled: false,
      content: {
        type: "Dropdown",
        value: {
          options: ["是", "否"],
          callback_fun_id: onEventIFStaringDay,
        },
      },
    },
    {
      node_id: "warn0",
      visibility: false,
      disabled: false,
      content: {
        type: "HtmlDocument",
        value: `
          <span style="display:inline-block;width:100%;text-align:center;font-weight:bold;color:red;"></span>
          `,
      },
    },
    {
      node_id: "send",
      visibility: true,
      disabled: false,
      content: {
        type: "Button",
        value: { primary: true, text: "发送", callback_fun_id: AddEventId },
      },
    },
    // 修改事件UI（13～25），警告23
    {
      node_id: "html3",
      visibility: true,
      disabled: false,
      content: {
        type: "HtmlDocument",
        value: `
          <span style="margin-left: 13%;">在这里选择你要修改的事件</span>
          `,
      },
    },
    {
      node_id: "tab1",
      visibility: true,
      disabled: false,
      content: {
        type: "Dropdown",
        value: {
          options: [],
          callback_fun_id: selectEventId,
        },
      },
    },
    {
      node_id: "html4",
      visibility: true,
      disabled: false,
      content: {
        type: "HtmlDocument",
        value: `
          <span style="margin-left: 13%;">输入事件名称</span>
          `,
      },
    },
    {
      node_id: "eventName2",
      visibility: true,
      disabled: false,
      content: {
        type: "Input",
        value: { text: event1.name, callback_fun_id: onEventName1 },
      },
    },
    {
      node_id: "html5",
      visibility: true,
      disabled: false,
      content: {
        type: "HtmlDocument",
        value: `
          <span style="margin-left: 13%;">输入事件目标日<span style="color:red;">（格式必须为YYYY-MM-DD）</span></span>
          `,
      },
    },
    {
      node_id: "eventTime2",
      visibility: true,
      disabled: false,
      content: {
        type: "Input",
        value: { text: event1.time, callback_fun_id: onEventTime1 },
      },
    },
    {
      node_id: "html311",
      visibility: true,
      disabled: false,
      content: {
        type: "HtmlDocument",
        value: `
          <span style="margin-left: 13%;">是否显示在主页</span>
          `,
      },
    },
    {
      node_id: "tab111",
      visibility: true,
      disabled: false,
      content: {
        type: "Dropdown",
        value: {
          options: ["是", "否"],
          callback_fun_id: onEventIndex1,
        },
      },
    },
    {
      node_id: "html321",
      visibility: true,
      disabled: false,
      content: {
        type: "HtmlDocument",
        value: `
          <span style="margin-left: 13%;">是否计入起始日</span>
          `,
      },
    },
    {
      node_id: "tab121",
      visibility: true,
      disabled: false,
      content: {
        type: "Dropdown",
        value: {
          options: ["是", "否"],
          callback_fun_id: onEventIFStaringDay1,
        },
      },
    },
    {
      node_id: "warn1",
      visibility: false,
      disabled: false,
      content: {
        type: "HtmlDocument",
        value: `
          <span style="display:inline-block;width:100%;text-align:center;font-weight:bold;color:red;"></span>
          `,
      },
    },
    {
      node_id: "get2",
      visibility: true,
      disabled: false,
      content: {
        type: "Button",
        value: { primary: true, text: "获取手环端数据", callback_fun_id: getEventId },
      },
    },
    {
      node_id: "send2",
      visibility: true,
      disabled: false,
      content: {
        type: "Button",
        value: { primary: true, text: "同步", callback_fun_id: ChangeEventId },
      },
    },
    // 删除事件UI（26～30），警告28
    {
      node_id: "html6",
      visibility: true,
      disabled: false,
      content: {
        type: "HtmlDocument",
        value: `
          <span style="margin-left: 13%;">在这里选择你要删除的事件</span>
          `,
      },
    },
    {
      node_id: "tab2",
      visibility: true,
      disabled: false,
      content: {
        type: "Dropdown",
        value: {
          options: [],
          callback_fun_id: selectEventId,
        },
      },
    },
    {
      node_id: "warn2",
      visibility: false,
      disabled: false,
      content: {
        type: "HtmlDocument",
        value: `
          <span style="display:inline-block;width:100%;text-align:center;font-weight:bold;color:red;"></span>
          `,
      },
    },
    {
      node_id: "get3",
      visibility: true,
      disabled: false,
      content: {
        type: "Button",
        value: { primary: true, text: "获取手环端数据", callback_fun_id: getEventId },
      },
    },
    {
      node_id: "send3",
      visibility: true,
      disabled: false,
      content: {
        type: "Button",
        value: { primary: true, text: "删除", callback_fun_id: DeleteEventId },
      },
    },
  ];

  Tab("添加事件");
});

async function AddEvent() {
  if (event.name == "") {
    warning("事件名称不能为空");
    return;
  } else if (!dayjs(event.time, "YYYY-MM-DD", true).isValid()) {
    warning("请输入正确的时间格式");
    return;
  }

  try {
    const appList = await AstroBox.thirdpartyapp.getThirdPartyAppList();
    const app = appList.find((app) => app.package_name == "com.yzf.daymatter");
    await AstroBox.thirdpartyapp.launchQA(app, "/pages/index");
    if (!app) {
      warning("请先安装倒数日快应用");
      return;
    } else if (app.version_code < 10400) {
      warning("请先安装倒数日快应用的新版本！");
      return;
    }

    await warning("正在发送，请稍等···");

    await AstroBox.interconnect.sendQAICMessage(
      "com.yzf.daymatter",
      JSON.stringify({
        type: "addEvent",
        name: event.name,
        date: event.time,
        on_index: event.on_index,
        IFStaringDay: event.IFStaringDay,
      })
    );
    warning("发送成功！");
  } catch (error) {
    console.error(error);
    warning(error);
  }
}

async function DeleteEvent() {
  if (index === "") {
    warning("请选择你要删除的事件！", 28);
    return;
  }

  try {
    const appList = await AstroBox.thirdpartyapp.getThirdPartyAppList();
    const app = appList.find((app) => app.package_name == "com.yzf.daymatter");
    await AstroBox.thirdpartyapp.launchQA(app, "/pages/index");
    if (!app) {
      warning("请先安装倒数日快应用", 28);
      return;
    } else if (app.version_code < 10400) {
      warning("请先安装倒数日快应用的新版本！", 28);
      return;
    }

    await warning("正在发送，请稍等···", 28);

    await AstroBox.interconnect.sendQAICMessage(
      "com.yzf.daymatter",
      JSON.stringify({
        type: "deleteEvent",
        index: index,
      })
    );

    getAllEvent.splice(index, 1);
    console.log(JSON.stringify(getAllEvent));
    ui[27].content.value.options = getAllEvent.map((item, index) => index + 1 + "　" + item.name);

    warning("发送成功！", 28);
  } catch (error) {
    console.error(error);
    warning(error, 28);
  }
}

async function ChangeEvent() {
  if (index === "") {
    warning("请选择你要修改的事件！", 23);
    return;
  }
  if (event1.name == "") {
    warning("事件名称不能为空", 23);
    return;
  } else if (!dayjs(event1.time, "YYYY-MM-DD", true).isValid()) {
    warning("请输入正确的时间格式", 23);
    return;
  }

  try {
    const appList = await AstroBox.thirdpartyapp.getThirdPartyAppList();
    const app = appList.find((app) => app.package_name == "com.yzf.daymatter");
    await AstroBox.thirdpartyapp.launchQA(app, "/pages/index");
    if (!app) {
      warning("请先安装倒数日快应用", 23);
      return;
    } else if (app.version_code < 10400) {
      warning("请先安装倒数日快应用的新版本！", 23);
      return;
    }

    await warning("正在发送，请稍等···", 23);

    await AstroBox.interconnect.sendQAICMessage(
      "com.yzf.daymatter",
      JSON.stringify({
        type: "changeEvent",
        name: event1.name,
        date: event1.time,
        on_index: event1.on_index,
        IFStaringDay: event1.IFStaringDay,
        index: index,
      })
    );

    getAllEvent[index].name = event1.name;
    getAllEvent[index].date = event1.time;
    ui[14].content.value.options[index] = index + 1 + "　" + event1.name;

    warning("发送成功！", 23);
  } catch (error) {
    console.error(error);
    warning(error, 23);
  }
}

async function getEvent() {
  let id;
  if (!whoGetEvent) {
    id = 23;
  } else {
    id = 28;
  }
  try {
    const appList = await AstroBox.thirdpartyapp.getThirdPartyAppList();
    const app = appList.find((app) => app.package_name == "com.yzf.daymatter");
    await AstroBox.thirdpartyapp.launchQA(app, "/pages/index");
    if (!app) {
      warning("请先安装倒数日快应用", id);
      return;
    } else if (app.version_code < 10400) {
      warning("请先安装倒数日快应用的新版本！", id);
      return;
    }

    await warning("正在发送，请稍等···", id);

    await AstroBox.interconnect.sendQAICMessage(
      "com.yzf.daymatter",
      JSON.stringify({
        type: "getAllEvent",
      })
    );
  } catch (error) {
    console.error(error);
    warning(error, id);
  }
}

AstroBox.event.addEventListener("onQAICMessage_com.yzf.daymatter", (data) => {
  let datas = JSON.parse(data);
  console.log(datas.data);
  getAllEvent = datas.data;
  let names = datas.data.map((item, index) => index + 1 + "　" + item.name);
  if (!whoGetEvent) {
    ui[14].content.value.options = names;
    ui[24].visibility = false;
    ui[25].visibility = true;
    warning("获取手环端数据成功！", 23);
  } else {
    ui[27].content.value.options = names;
    ui[29].visibility = false;
    ui[30].visibility = true;
    warning("获取手环端数据成功！", 28);
  }
});
