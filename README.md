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

表单可选择公历/农历，切换保持同一天。农历年月日按YYYY-MM-DD输入，闰月另选；范围1900～2100。原始无效日期拒绝发送，年度没有对应闰月按普通同月，三十遇小月按月末。农历与原始对应公历分别保存，表单说明显示公历核对值（输入后再次刷新/切换控件或发送时校验）。需要两端确认农历能力和dm-hko-v1换算表版本。

添加页提供生日、考试、纪念日模板，设置分类、重复规则和天数单位；日期与自定义名称保留，空名称或与任一模板默认名称完全相同的名称随模板切换，只更新草稿，仍需确认发送。生日与纪念日默认年度重复，考试默认单次。

1. 顶部切到「添加」
2. 在「事件信息」卡片填写事件名称、目标日期（默认今天）
3. 设置「显示在快应用首页」与「计入起始日」（两项影响快应用，**不影响表盘**）
4. 点击底部「发送到腕端」

新增/修改表单可设置「每年重复」和显示单位（`days` / `weeks` / `months` / `years`）。年度重复保留原始日期，当日仍为本次，非闰年2月29日按2月28日发生；列表展示下次日期和周年数。新字段要求腕端确认yearlyRepeat/displayUnit能力，未确认且使用非默认值时停止发送并提示更新。显示单位独立持久化，不改变表盘导出格式。

### 修改事件

列表显示范围可循环正常事件、已归档、今天事项、未来30天（含今天）。年度事件按本次/下次发生日筛选，过去单次事件和归档不进入近期；分类、排序和稳定ID分页规则继续适用。

第二批支持置顶、固定分类（未分类/生日/学习/生活/纪念日）与归档；表单显示单位与分类使用中文下拉选择框，内部保存对应协议标识。修改/删除列表可切换分类及正常/已归档；每页仍最多8条。排序卡片循环创建/临近/手动，并在腕端保存成功后切换显示；同步列表读取腕端持久模式。

编辑子页「前移/后移」移动已保存事件，只在同置顶组及同归档状态内移动，不受分类筛选限制；未提交草稿保留，需先同步修改以应用草稿。移动后重新同步腕端列表查看结果。置顶不改变表盘主事件；归档主事件由第一个未归档首页事件接替，无候选清空，恢复不自动夺回。

新字段与操作按pinned/category/archived/manualOrder/sortMode逐项能力门控；未确认时不发送非默认新字段或新操作，提示更新腕端。

1. 顶部切到「修改」，点击「同步腕端数据」
2. 在事件列表中点击目标事件卡片，进入编辑子页（左上角圆形按钮返回概览）
3. 修改名称 / 日期 / 显示选项
4. 点击「同步修改」，成功后自动回到概览

### JSON 备份与恢复

在「修改」页先同步完整腕端列表，再点击「导出完整 JSON 备份」保存所有事件、未知扩展、排序模式与主事件引用；包括归档事件，不受分类和分页过滤影响。

选择 JSON 后先校验并预览，显示同 ID 冲突及同名不同 ID。合并可选择保留现有或使用备份的同 ID 记录，新 ID 追加，保留当前排序与主事件；替换采用备份集合、排序和主事件。同名不同 ID 不按名称覆盖。

文件最多240 KiB、200条，快照60秒后需要刷新。预览后设备或数据改变要求重新预览，腕端提交时再次核对revision。事件写入失败保留旧数据与预览；最终回执成功后重新同步核对。背景图片和未提交草稿不属于事件备份。恢复要求新腕端确认backupRestore能力，不退回旧逐条新增。

### 表盘主事件

在「修改」页同步腕端数据，打开事件编辑子页，点击「设为表盘主事件」或当前主事件的「取消表盘主事件」。该操作只修改已保存事件的表盘引用，不提交名称等编辑草稿，与首页展示、置顶、排序独立。

设置会替换原主事件，取消会清空表盘且不自动指定替代。归档事件不能指定，需先恢复并同步。取消时腕端核对原主事件 ID，若已变更会拒绝，避免旧列表误清空新主事件。

需要两端更新并确认 `primaryMutation`、`primaryId`、稳定 ID 及保存回执能力。当前标记以腕端最终回执为准；主事件保存成功但表盘文件失败时单独提示待补写，无需重复设置。消息发送成功不等于保存成功。

### 删除事件

1. 顶部切到「删除」，点击「同步腕端数据」（若尚未获取）
2. 在事件列表中点击选中事件，脚注会回显「已选「名称 · 目标日」」
3. 点击「删除所选事件」，在确认框中核对名称与日期后确认

### 状态提示

状态面板常驻在分段控制器上方，分「进行中 / 成功 / 失败」三态，点击可关闭；
传输完成只显示等待确认；协商结果协议后，以腕端提交回执显示「腕端已保存」或明确失败。
事件保存与主事件/表盘同步分别报告，分项失败时不要重复新增。
超时查询原请求后仍接收迟到回执；刷新列表不会解锁尚未确认的新协议写操作。
原结果未知/过期时保留待核实状态；先查询并同步同设备完整列表，核对后点击
“已核实，结束等待”显式恢复。该操作不声明腕端确认成功，不重发增删改，并保留草稿。
核实快照 60 秒后过期；仍在 processing 时不能直接解除等待。原结果查询 8 秒无响应后，
须再次同步同设备完整列表，再显式核实结束等待；不会自动重发。原请求未结算时拒绝换设备核实。

每次业务发送先等待能力握手（最多约 2 秒），超时使用旧业务模式；只有同时确认
requestId 与 mutationResult 才启用保存结果协议。带稳定 ID 的修改/删除还要求 stableId 能力。
操作启动前固定设备，修改/删除核对列表所属设备；连接变化后应先重新同步列表。
能力回包还须回显本次 `handshakeId` 和设备关联字段；未回显的旧腕端等待超时后走旧模式。
带稳定 ID 的修改/删除不会在未确认能力时静默降级，建议快应用和插件同步更新。

双方确认 `listRequestId` 后，列表回包按请求、会话、设备及 revision 校验；新拉取、
超时或已完成请求的迟到回包不会覆盖快照。列表等待窗口为 8 秒，刷新保留未提交草稿。
旧腕端无请求身份时仅在拉取窗口内兼容接收，无法区分该窗口内所有迟到旧列表。
异常条目跳过后仍保留原数组下标；旧下标操作的快照有效期为 60 秒，过期需重新同步。
接收消息最多 256KiB、列表最多 200 条；重复 ID 或非法列表元数据会拒绝替换现有列表。
双方确认 `listBatches` 后，列表按每批最多 16 条/8KiB 回传；插件允许乱序和相同重复批次，
全部批次验证完成后才替换列表，缺批、冲突或超预算保留旧列表及草稿。

### 背景图片

最新上传按实际设备原尺寸生成（单边≤512px；9 Pro为336×480），JPEG≤100KiB，通过3KiB二进制小分片逐片落盘ACK，长度/校验后提交背景引用并等待最终回执；要求最新腕端backgroundChunks能力。原450px/整包方案仅为历史，预览内容仍与成品一致。

1. 顶部切到「背景」，自动从连接设备的最新版腕端读取实际屏幕宽高；可点击「读取连接设备屏幕」重读。尺寸未知时先更新腕端再试，选择居中裁剪填满或等比留黑边。
2. 选图后先检查真实JPEG/PNG格式、文件≤16MiB、单边≤8192px、≤8Mi像素，解码器分配预算64MiB；缩放到最长边450px，输出不超过100KiB的JPEG，透明PNG合成白底。
3. 选择图片先生成实际JPEG成品预览，确认后点击「发送预览成品到腕端」才上传。修改参数使旧预览失效，需要重新选图；原图不常驻，圆屏/胶囊边缘由设备裁切，data-URI预览宿主渲染待验收。
4. 腕端按实际编码保存`.jpg`/`.png`并兼容旧`.png`文件；新图及背景索引提交后才清理旧图，保存失败留旧背景。插件发送成功仍须在腕端核实显示，尚无背景保存回执。

选图宿主接口一次性返回完整文件，16MiB检查在插件收到文件后、图片解码前执行，不能限制宿主读取阶段的内存。64MiB是解码器限制，不代表整体实测峰值。腕端另限输入100KiB和编码头450px；真实JPEG显示须按设备/固件验收，不支持JPEG的固件尚无自动PNG回退。

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

当前 native 测试包含实际跨端编码验证：需要 Node.js 与相邻目录 `../daymatter` 仓库。
`src/cross_protocol_tests.rs` 调用该仓库的 `tools/protocol-bridge.cjs`，用 Rust 编码请求
驱动腕端实际页面脚本，再将腕端回包交给 Rust 解码和状态机。宿主 UI 与真实连接仍需设备验证。

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
因此发布计划要求保留上一版 `api_level 2` 插件包，直到确认用户宿主已升级；当前仓库仅找到 Level4 ABP，真实回退产物仍待取得与校验；
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
