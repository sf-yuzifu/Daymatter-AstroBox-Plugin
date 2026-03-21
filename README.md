# 倒数日配置工具

一个基于 Rust 和 WASI/Component Model 开发的 AstroBox V2 插件，用于快速管理手环倒数日事件。

## 功能特性

- ✨ **添加事件**：快速添加新的倒数日事件，支持自定义名称、日期、显示设置等
- 🔄 **修改事件**：修改已存在的倒数日事件，实时同步到手环
- 🗑️ **删除事件**：删除不需要的倒数日事件
- 📱 **数据同步**：从手环端获取所有倒数日事件数据
- ✅ **表单验证**：完善的输入验证，提供清晰的错误提示
- ⏱️ **自动隐藏提示**：提示信息 3 秒后自动隐藏，提升用户体验
- 🎨 **交互优化**：按钮悬停效果、下拉菜单自动填充等流畅交互

## 技术栈

- **语言**：Rust (Edition 2024)
- **架构**：WASI (WebAssembly System Interface) / Component Model
- **UI 框架**：AstroBox PSYS Host UI
- **通信**：Interconnect 消息通信
- **定时器**：Timer API
- **依赖**：
  - `wit-bindgen` (0.47.0) - WIT 接口绑定
  - `serde_json` (1.0) - JSON 解析
  - `chrono` (0.4) - 日期处理
  - `tracing` (0.1) - 日志记录

## 安装

### 前置要求

- Rust 工具链 (Edition 2024)
- Python 3.x
- AstroBox V2 平台

### 克隆项目

```bash
git clone https://github.com/sf-yuzifu/Daymatter-AstroBox-Plugin.git
cd Daymatter-AstroBox-Plugin
```

### 更新子模块

```bash
# Windows
update_submodules.bat

# Linux/macOS
./update_submodules.sh
```

## 构建

### 构建插件

```bash
python scripts/build_dist.py
```

构建完成后，生成的 WASM 文件位于 `dist` 目录。

### 打包插件

```bash
python scripts/build_dist.py --release --package
```

构建完成后，生成的 ABP 文件位于 `dist` 目录。

### 安装到 AstroBox

将生成的 `daymatter_astrobox_v2_plugin.wasm` 文件和 `manifest.json` 放置到 AstroBox 插件目录中。

## 使用说明

### 添加事件

1. 进入"添加事件"标签页
2. 填写事件名称
3. 选择目标日期（默认为当前日期）
4. 设置是否显示在主页
5. 设置是否计入起始日
6. 点击"添加事件"按钮

### 修改事件

1. 进入"修改事件"标签页
2. 点击"获取手环端数据"按钮
3. 从下拉菜单中选择要修改的事件
4. 修改事件信息
5. 点击"修改事件"按钮

### 删除事件

1. 进入"删除事件"标签页
2. 点击"获取手环端数据"按钮
3. 从下拉菜单中选择要删除的事件
4. 点击"删除事件"按钮

## 项目结构

```
Daymatter-AstroBox-Plugin/
├── src/
│   ├── ui/
│   │   ├── mod.rs           # UI 模块入口
│   │   ├── state.rs         # 状态管理
│   │   ├── message.rs       # 消息处理和通信
│   │   ├── validation.rs    # 表单验证
│   │   ├── event_handler.rs # 事件处理
│   │   ├── build.rs         # UI 构建
│   │   └── device.rs       # 设备相关功能
│   ├── lib.rs              # 插件入口
│   └── logger.rs           # 日志配置
├── wit/                    # WIT 接口定义
├── scripts/
│   └── build_dist.py       # 构建脚本
├── Cargo.toml             # Rust 项目配置
├── manifest.json           # 插件清单
└── icon.png               # 插件图标
```

## 开发指南

### 状态管理

项目使用 `RwLock` 和 `OnceLock` 实现线程安全的状态管理：

```rust
static UI_STATE: OnceLock<RwLock<UiState>> = OnceLock::new();

pub fn ui_state() -> &'static RwLock<UiState> {
    UI_STATE.get_or_init(|| {
        RwLock::new(UiState { /* ... */ })
    })
}
```

### 事件处理

所有 UI 事件通过 `ui_event_processor` 函数统一处理：

```rust
pub fn ui_event_processor(evtype: ui::Event, event: &str, event_payload: &str) {
    match evtype {
        ui::Event::Click => handle_button_click(event),
        ui::Event::Change => handle_input_event(event, event_payload),
        ui::Event::MouseEnter => handle_mouse_enter(event),
        ui::Event::MouseLeave => handle_mouse_leave(event),
        _ => {}
    }
}
```

### Timer API

使用 Timer API 实现定时功能：

```rust
// 设置定时器
let timer_id = timer::set_timeout(3000, "hide_message").await;

// 清除定时器
timer::clear_timer(timer_id).await;

// 处理定时器事件
EventType::Timer => {
    let payload = extract_payload_text(&event_payload);
    if payload == "hide_message" {
        hide_message();
    }
}
```

### Interconnect 通信

通过 Interconnect API 与手环应用通信：

```rust
// 注册接收器
register::register_interconnect_recv(device_addr, "com.yzf.daymatter").await;

// 发送消息
interconnect::send_qaic_message(device_addr, "com.yzf.daymatter", payload).await;

// 处理接收到的消息
pub fn handle_interconnect_message(payload: &str) {
    // 解析并处理消息
}
```

## 权限

插件需要以下权限：

- `interconnect` - 与手环应用通信
- `thirdpartyapp` - 访问第三方应用
- `device` - 访问设备信息
- `register_interconnect_recv` - 注册消息接收器

## 版本要求

- **WASI 版本**：2
- **API 级别**：2
- **手环应用**：倒数日快应用 (版本 >= 21000)

## 许可证

本项目采用 Apache License 2.0 许可证。详见 [LICENSE](LICENSE) 文件。

## 作者

小鱼yuzifu

## 网站

https://github.com/sf-yuzifu/Daymatter-AstroBox-Plugin

## 贡献

欢迎提交 Issue 和 Pull Request！

## 致谢

感谢 AstroBox 团队提供的开发框架和[文档](https://plugindoc-v2.astrobox.online/)支持。
