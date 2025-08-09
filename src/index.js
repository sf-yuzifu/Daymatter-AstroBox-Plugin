import AstroBox from "astrobox-plugin-sdk";
import dayjs from "dayjs";
import customParseFormat from "dayjs/plugin/customParseFormat";

let ui;
let getAllEvent = [];
let index;

const event = {
  name: "",
  time: dayjs().format("YYYY-MM-DD"),
};

const event1 = {
  name: "",
  time: "",
};

dayjs.extend(customParseFormat);

// UI服务启动
let onEventName = AstroBox.native.regNativeFun(eventName);
let onEventTime = AstroBox.native.regNativeFun(eventTime);
let onEventName1 = AstroBox.native.regNativeFun(eventName1);
let onEventTime1 = AstroBox.native.regNativeFun(eventTime1);
let AddEventId = AstroBox.native.regNativeFun(AddEvent);
let onTab = AstroBox.native.regNativeFun(Tab);
let getEventId = AstroBox.native.regNativeFun(getEvent);
let selectEventId = AstroBox.native.regNativeFun(selectEvent);
let ChangeEventId = AstroBox.native.regNativeFun(ChangeEvent);

async function selectEvent(text) {
  let space = text.indexOf("　");
  index = parseInt(text.slice(0, space)) - 1;
  event1.name = getAllEvent[index].name;
  event1.time = getAllEvent[index].date;
  console.log(JSON.stringify(event1));
  ui[12] = {
    node_id: "eventName2",
    visibility: true,
    disabled: false,
    content: {
      type: "Input",
      value: { text: event1.name, callback_fun_id: onEventName1 },
    },
  };
  ui[14] = {
    node_id: "eventTime2",
    visibility: true,
    disabled: false,
    content: {
      type: "Input",
      value: { text: event1.time, callback_fun_id: onEventTime1 },
    },
  };
  ui[12].visibility = false;
  ui[14].visibility = false;
  AstroBox.ui.updatePluginSettingsUI(ui);
  await sleep(100);
  ui[12].visibility = true;
  ui[14].visibility = true;
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
      for (let i = 2; i <= 8; i++) {
        if (i != 7) {
          ui[i].visibility = true;
        }
      }
      break;
    case "修改事件":
      for (let i = 9; i <= 16; i++) {
        if (i != 15) {
          ui[i].visibility = true;
        }
      }
      getAllEvent = [];
      event1.name = "";
      event1.time = "";
      index = "";
      ui[10].content.value.options = [];
      ui[12] = {
        node_id: "eventName2",
        visibility: true,
        disabled: false,
        content: {
          type: "Input",
          value: { text: event1.name, callback_fun_id: onEventName1 },
        },
      };
      ui[14] = {
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
      break;
    default:
      break;
  }
  AstroBox.ui.updatePluginSettingsUI(ui);
}

async function eventName(text) {
  event.name = text;
  ui[4] = {
    node_id: "eventName",
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
    node_id: "eventTime",
    visibility: true,
    disabled: false,
    content: {
      type: "Input",
      value: { text: event.time, callback_fun_id: onEventTime },
    },
  };
  AstroBox.ui.updatePluginSettingsUI(ui);
}

async function eventName1(text) {
  event1.name = text;
}

async function eventTime1(text) {
  event1.time = text;
}

const sleep = (delay) => new Promise((resolve) => setTimeout(resolve, delay));

async function warning(warn, type = 7) {
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
    // 添加事件UI（2～8）
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
    // 修改事件UI（9～17）
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
    } else if (app.version_code < 10300) {
      warning("请先安装倒数日快应用的版本！");
      return;
    }

    await warning("正在发送，请稍等···");

    await AstroBox.interconnect.sendQAICMessage(
      "com.yzf.daymatter",
      JSON.stringify({
        type: "addEvent",
        name: event.name,
        date: event.time,
        on_index: true,
      })
    );
    warning("发送成功！");
  } catch (error) {
    console.error(error);
    warning(error);
  }
}

async function ChangeEvent() {
  if (index == "") {
    warning("请选择你要修改的事件！", 15);
    return;
  }
  if (event1.name == "") {
    warning("事件名称不能为空", 15);
    return;
  } else if (!dayjs(event1.time, "YYYY-MM-DD", true).isValid()) {
    warning("请输入正确的时间格式", 15);
    return;
  }

  try {
    const appList = await AstroBox.thirdpartyapp.getThirdPartyAppList();
    const app = appList.find((app) => app.package_name == "com.yzf.daymatter");
    await AstroBox.thirdpartyapp.launchQA(app, "/pages/index");
    if (!app) {
      warning("请先安装倒数日快应用", 15);
      return;
    } else if (app.version_code < 10300) {
      warning("请先安装倒数日快应用的版本！", 15);
      return;
    }

    await warning("正在发送，请稍等···", 15);

    await AstroBox.interconnect.sendQAICMessage(
      "com.yzf.daymatter",
      JSON.stringify({
        type: "changeEvent",
        name: event1.name,
        date: event1.time,
        on_index: true,
        index: index,
      })
    );
    warning("发送成功！", 15);
  } catch (error) {
    console.error(error);
    warning(error);
  }
}

async function getEvent() {
  try {
    const appList = await AstroBox.thirdpartyapp.getThirdPartyAppList();
    const app = appList.find((app) => app.package_name == "com.yzf.daymatter");
    await AstroBox.thirdpartyapp.launchQA(app, "/pages/index");
    if (!app) {
      warning("请先安装倒数日快应用", 15);
      return;
    } else if (app.version_code < 10300) {
      warning("请先安装倒数日快应用的版本！", 15);
      return;
    }

    await warning("正在发送，请稍等···", 15);

    await AstroBox.interconnect.sendQAICMessage(
      "com.yzf.daymatter",
      JSON.stringify({
        type: "getAllEvent",
      })
    );
  } catch (error) {
    console.error(error);
    warning(error, 15);
  }
}

AstroBox.event.addEventListener("onQAICMessage_com.yzf.daymatter", (data) => {
  let datas = JSON.parse(data);
  console.log(datas.data);
  getAllEvent = datas.data;
  let names = datas.data.map((item, index) => index + 1 + "　" + item.name);
  ui[10].content.value.options = names;
  warning("获取手环端数据成功！", 15);
  ui[16].visibility = false;
  ui[17].visibility = true;
});
