import AstroBox from "astrobox-plugin-sdk";
import dayjs from "dayjs";
import customParseFormat from "dayjs/plugin/customParseFormat";

let ui;

const event = {
  name: "",
  time: dayjs().format("YYYY-MM-DD"),
};

dayjs.extend(customParseFormat);

// UI服务启动
let onEventName = AstroBox.native.regNativeFun(eventName);
let onEventTime = AstroBox.native.regNativeFun(eventTime);
let AddEventId = AstroBox.native.regNativeFun(AddEvent);

async function eventName(text) {
  event.name = text;
}

async function eventTime(text) {
  event.time = text;
}

const sleep = (delay) => new Promise((resolve) => setTimeout(resolve, delay));

async function warning(warn, time = 2000) {
  ui[5].content.value = `
          <span style="display:inline-block;width:100%;text-align:center;font-weight:bold;color:red;">${warn}</span>
          `;
  ui[5].visibility = true;
  AstroBox.ui.updatePluginSettingsUI(ui);
  await sleep(time);
  ui[5].visibility = false;
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
          <span style="display:inline-block;width:100%;text-align:center;font-size:24px;font-weight:bold;">添加事件</span>
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
      node_id: "eventName",
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
      node_id: "eventTime",
      visibility: true,
      disabled: false,
      content: {
        type: "Input",
        value: { text: event.time, callback_fun_id: onEventTime },
      },
    },
    {
      node_id: "warn",
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
  ];

  AstroBox.ui.updatePluginSettingsUI(ui);
});

// AstroBox.lifecycle.onUILoad(() => {
//   ui[2].content.value.text = event.name;
//   ui[4].content.value.text = event.time;
// });

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

    await warning("正在发送，请稍等···", 2000);

    await AstroBox.interconnect.sendQAICMessage(
      "com.yzf.daymatter",
      JSON.stringify({
        name: event.name,
        date: event.time,
        on_index: true,
      })
    );
    warning("发送成功！");
  } catch (error) {
    console.error(error);
    warning(error.message);
  }
}
