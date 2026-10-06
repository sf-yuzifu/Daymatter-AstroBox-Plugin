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

界面沿用与腕上漫画插件一致的 ClaraLight 规范：顶部药丸分段控制器、18px 大圆角卡片、
概览页与独立编辑子页、底部单一主操作、常驻状态面板、发送期间锁定重复操作。

### 添加事件

1. 顶部切到「添加」
2. 在「事件信息」卡片填写事件名称、目标日期（默认今天）
3. 设置「显示在快应用首页」与「计入起始日」（两项影响快应用，**不影响表盘**）
4. 点击底部「发送到腕端」

### 修改事件

1. 顶部切到「修改」，点击「同步腕端数据」
2. 在事件列表中点击目标事件卡片，进入编辑子页（左上角圆形按钮返回概览）
3. 修改名称 / 日期 / 显示选项
4. 点击「同步修改」，成功后自动回到概览

### 删除事件

1. 顶部切到「删除」，点击「同步腕端数据」（若尚未获取）
2. 在事件列表中点击选中事件，脚注会回显「已选「名称 · 目标日」」
3. 点击「删除所选事件」，在确认框中核对名称与日期后确认

### 状态提示

状态面板常驻在分段控制器上方，分「进行中 / 成功 / 失败」三态，点击可关闭；
文案只声明「已发送，等待腕端确认」，是否真正落盘以腕端提示为准。

### 背景图片

1. 顶部切到「背景」，点击「选择背景图片」
2. 选图后自动压缩（最长边 450px、不超过 100KB 的 JPEG）并发送到腕端
3. 新图未确认前不会替换腕端当前背景

## 项目结构

```
Daymatter-AstroBox-Plugin/
├── src/
│   ├── lib.rs              # 插件入口（wasm32 才编译绑定与 guest 实现）
│   ├── guest.rs            # 宿主导出实现（Level 4 全 async）
│   ├── logger.rs           # 日志配置
│   └── ui/
│       ├── mod.rs          # 模块入口（宿主层按 target 条件编译）
│       ├── state.rs        # 状态与选择键（纯逻辑）
│       ├── dispatch.rs     # 事件标识表与动作派发（纯逻辑）
│       ├── view.rs         # 视图模型与重建指纹（纯逻辑）
│       ├── theme.rs        # 设计令牌与配色决策（纯逻辑）
│       ├── protocol.rs     # 两端消息编解码（纯逻辑）
│       ├── validation.rs   # 表单校验（纯逻辑）
│       ├── build.rs        # 宿主渲染层
│       ├── event_handler.rs# 宿主事件入口
│       ├── message.rs      # 状态条 / 定时器 / 设备通道
│       └── image.rs        # 背景图片压缩
├── wit/                    # WIT 接口定义（psys-world-v4）
├── scripts/
│   └── build_dist.py       # 构建脚本
├── Cargo.toml             # Rust 项目配置
├── rust-toolchain.toml    # 工具链固定（1.98.0 + wasm32-wasip2）
├── manifest.json           # 插件清单
└── icon.png               # 插件图标
```

## 开发指南

### 分层原则

纯逻辑层（`state` / `dispatch` / `view` / `theme` / `protocol` / `validation`）不引用任何宿主接口，
因此可以在开发机上直接跑测试；宿主层（`build` / `event_handler` / `message` / `image`）只在
`wasm32` 目标下编译，负责把视图模型翻译成元素树、执行宿主调用。

```bash
# 逻辑层测试（46 项，不需要 AstroBox 宿主）
cargo test --target x86_64-pc-windows-msvc

# 产物构建
cargo build --release
python scripts/build_dist.py --release
```

### 状态管理

状态用 `OnceLock<RwLock<UiState>>` 保存，`UiState` 只放纯数据：

```rust
pub fn read_state() -> RwLockReadGuard<'static, UiState>;
pub fn write_state() -> RwLockWriteGuard<'static, UiState>;
pub fn reset_state(today: &str); // 宿主加载时对齐时区，测试时隔离用例
```

> **锁纪律（重要）**：`std::sync::RwLock` 不可重入。状态锁内只构造渲染请求，释放 guard 后再调用宿主 `ui::render`；
> 持锁期间调用 `build::refresh()` 会重复获取写锁，造成死锁。所有发送使用带 token 的单飞状态机，
> 不用跨宿主 `await` 持有全局业务锁，否则等待设备/API 时会把其他 UI 事件一并堵住。

事件选择使用**稳定键**（`id:<腕端ID>` 或 `idx:<下标>`），不用显示序号定位，
列表刷新后按键重新校验，键失效即提示重新选择，不会误改同名事件。

### 事件处理

事件标识由 `dispatch::UiId` 单一表格定义，格式为 `{区域}.{scope}.{角色}`
（如 `field.modify.on_home.yes`、`action.delete.submit`、`pick.modify.id:e1`）；
`dispatch::resolve` 把标识与载荷解析成 `Action`，`dispatch::apply_action` 变换状态并
返回"是否需要重建界面"。测试会双向核对：每个标识都有动作分支，每个被绑定的标识都能反解。

```rust
let action = dispatch::resolve(event_id, kind, payload);   // 纯函数
let change = dispatch::apply_action(&mut state, action);   // 纯函数
if change.rebuild { build::refresh(false); }               // 指纹未变则跳过重画
```

### 界面重建

宿主只提供根元素渲染（`ui.render(root_id, element)`），因此优化方式是**避免无谓重建**：
`view::should_render` 用视图指纹（FNV-1a，覆盖分段 / 状态 / 卡片 / 列表 / 表单 / 主操作）判断，
指纹未变直接跳过；输入事件只改草稿、不触发重建；按钮不绑定 hover 事件，
避免每次鼠标移动都重画整棵树。

### UI/UX 规范（ClaraLight，与腕上漫画插件同源）

| 要素 | 规范 |
| --- | --- |
| 顶层导航 | `segmented_control` 药丸底座 + 34px 互斥高亮药丸，未选中段才可点 |
| 页面结构 | 概览页（说明卡 + 列表）+ 独立编辑子页（36×36 圆形返回 + 19px 标题） |
| 卡片 | 18px 大圆角、`PANEL #1E1E1F` 底、淡描边、`CONTROL` 半透明白底 |
| 主操作 | 每页恰好一个，次要操作降级为 `AccentQuiet` 并排 |
| 状态面板 | 三态常驻（进行中 / 成功 / 失败），带图标、点击关闭，不自动消失 |
| 在途锁定 | `pending_send` 存在时主操作禁用，重复派发静默忽略 |
| 二元开关 | 「是 / 否」两项始终可点击，选中态只改变强调色，不会禁用当前项或另一项 |
| 图标 | 内联 Phosphor 风格 SVG（plus / edit / trash / image / calendar / watch / back / chevron） |
| 窄屏 | `width_full` + `min_width(0)` + `overflow-wrap: anywhere`；列表每页 8 条 + 分页器 |

令牌与纯样式决策集中在 `src/ui/theme.rs`，元素拼装集中在 `src/ui/widgets.rs`，
结构与文案决策集中在 `src/ui/view.rs`（纯逻辑，可在 host 上测试）。

### Timer API

等待由宿主定时器事件推进（`set_timeout` 只注册定时器并立即返回，没有 async sleep）：

```rust
// 启动快应用后登记延时发送，取代 thread::sleep
timer::set_timeout(APP_LAUNCH_SETTLE_MS, SEND_TIMER);

// 定时器事件里继续推进，失败有界重试
event::EventType::Timer => ui::handle_timer_event(&event_payload).await,
```

### Interconnect 通信

```rust
register::register_interconnect_recv(device_addr.clone(), PKG_NAME.to_string()).await;
interconnect::send_qaic_message(device_addr, PKG_NAME.to_string(), payload).await;
```

消息体一律用 `serde_json` 构造（`protocol.rs`），名称中的引号 / 反斜杠 / 换行不会破坏消息；
解析腕端返回时单条异常只跳过该条并计数，不让整批数据失败。

## 权限

插件需要以下权限：

- `interconnect` - 与手环应用通信
- `thirdpartyapp` - 访问第三方应用
- `device` - 访问设备信息
- `register_interconnect_recv` - 注册消息接收器
- `dialog` - 删除确认对话框

## 版本要求

- **WASI 版本**：3（组件模型，目标仍为 `wasm32-wasip2`）
- **API 级别**：4（要求支持 API Level 4 的 AstroBox V4 宿主）
- **手环应用**：倒数日快应用 (版本 >= 21000)
- **工具链**：Rust 1.98.0（见 `rust-toolchain.toml`）

### 宿主门槛与降级

Level 4 的导出全部是 `async func`，旧宿主（仅支持 Level 2 / 3）无法加载本插件。
因此发布时保留上一版 `api_level 2` 插件包，直到确认用户宿主已升级；
快应用侧不感知插件版本，协议字段（`id` / `primaryId` / `revision`）对旧插件均为可选增量。

本仓库没有 AstroBox V4 的具体最低语义化版本号，不能据 API Level 推测版本号；安装前应以宿主插件管理器报告的 API Level 4 能力为准。发布策略是保留上一版 Level 2 ABP 作为独立回退包，新版只发布给 Level 4 宿主；Level 4 插件启动后与腕端协商 `protocolVersion: 2` 和逐项 capabilities，未收到能力确认时保持旧业务 JSON，不发送新协议字段。快应用版本门槛为 `versionCode >= 21000`。

同步协议 v2 使用 `requestId` + `sessionId`，腕端只在正式事件文件提交后回报 mutation 结果；查询原请求不会重放增删改。结果缓存限制 32 条，随快应用进程销毁失效，重启后须重新拉取事件列表。当前 interconnect 消息未暴露可信发送方设备地址，因此回包用会话与 requestId 关联，并要求操作前复核当前连接设备；不能将 JSON 内自报的 deviceId 当作身份认证。

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
