# ADR-0006：自定义标题栏与一体化窗口工作带

- 状态：Accepted
- 日期：2026-11-06
- 决策者：项目负责人（用户）与主开发助手
- 相关文档：`docs/design/UI_REDESIGN_PHASE_3_6.md`、`docs/design/DESIGN_SYSTEM.md`、`docs/design/UI_AUDIT.md`
- 取代：ADR-0005（保留原生窗口装饰，采用一体化工作区外壳）
- 上游决策：ADR-0001（Tauri 2 本地优先架构）

## 背景

Phase 3.5 保留了 Windows 原生标题栏，移除了应用级顶栏，并用 240px 侧栏与 48px `PageHeader` 组成“单一工作带”。该方案保留了全部原生窗口行为，但视觉审查确认仍存在明显的两层结构：

1. Windows 原生标题栏由系统绘制，背景、字体与焦点样式不受 XUSHU 设计系统控制；
2. 标题栏下方才是应用内容，顶部无法形成 Obsidian 式的连续工作区；
3. 侧栏品牌行虽然与页面标题行高度一致，但无法与窗口顶端真正衔接；
4. 工作台仍以“工作台”页面标题开场，保留管理后台的观感。

Phase 3.6 的任务不是继续局部换色，而是从窗口结构层面重新评估自定义标题栏。用户明确要求：优先研究 Tauri 2 在 Windows 下的自定义标题栏能力；如果原生拖动、双击、最小化/最大化/关闭、Snap Layouts、DPI 与焦点行为明显退化，则退回保守增强方案。

## 调研结论

### Tauri 2 能力

Tauri 2 支持：

- `decorations: false` 的无装饰窗口；
- `data-tauri-drag-region`（Tauri 2.12 支持 `"deep"` 子树拖动）；
- `internal_toggle_maximize`、`start_dragging`、`minimize`、`toggle_maximize`、`close` 等窗口命令；
- `shadow: true` 在 Windows 11 上提供窗口阴影与圆角提示；
- `set_theme` 窗口主题同步。

但 Tauri 2.12 没有提供 Windows 原生 Snap Layouts 的一等 API。Tauri 上游 issue #4531 仍为开放状态，维护者说明该能力受 WebView2 限制，未在 Tauri v2 实现。

### WebView2 命中测试限制

设置 `decorations: false` 后，WebView2 子窗口覆盖整个客户区并先接收鼠标消息。只在 Tauri 主窗口的 `WM_NCHITTEST` 中返回 `HTMAXBUTTON` 不会生效，因为主窗口不是命中测试的第一个接收者。

Microsoft 对 Win32 自定义标题栏的官方要求是：在最大化/还原按钮区域返回 `HTMAXBUTTON`，Windows 11 才会显示 Snap Layouts 悬停菜单。社区可行的实现方式是在最大化按钮位置创建一个**不可见的原生子窗口**，由该子窗口返回 `HTMAXBUTTON`，并把点击与悬停回传应用。

### 评估过的第三方实现

| 方案 | 结论 |
|---|---|
| `tauri-plugin-decorum` | 可用，但 Windows 路径会注入自带 HTML 标题栏，并依赖 `withGlobalTauri` 与插件权限；与本项目自定义 React 工作带重叠。 |
| `tauri-plugin-frame` | 基于原生子窗口实现 Snap 覆盖层，方向正确；但包含自动标题栏、CSS 变量、悬停配色等额外表面，且需要额外插件依赖。 |
| 自行实现最小 Snap 覆盖层 | 采用。只保留一个不可见子窗口、`HTMAXBUTTON`、点击/悬停事件与 DPI 定位，代码量可控，不改变前端控件与安全模型。 |

## 决策

### 1. 启用自定义标题栏

`tauri.conf.json`：

- `decorations: false`
- `shadow: true`
- `minWidth: 500`
- `minHeight: 480`
- 保留原生窗口标题（任务栏/辅助功能仍可识别“序枢 XUSHU”）。

窗口顶部不再由系统绘制标题栏，而是由 XUSHU 的 44px 工作带承担：

```text
┌──────────────────────────────────────────────────────────────────────┐
│ 序 序枢 XUSHU │ 当前项目 [项目 ▾]  branch/HEAD ● │ 工作台 │ ☀ ☾ ◐ │ – □ × │ 44px
├───────────────┼──────────────────────────────────────────────────────┤
│ 导航          │ 页面内容                                             │
│ ...           │                                                      │
└───────────────┴──────────────────────────────────────────────────────┘
```

- 左侧 240px 品牌区与侧栏同色、同宽，窄窗口（<760px）收为 52px 图标轨。
- 项目上下文、分支/HEAD/dirty、页面名称、主题与窗口控制位于同一视觉表面。
- 窗口控制为 46×44px 的自绘按钮，右侧贴边；关闭按钮悬停使用 Windows 红。
- 顶部工作带与主内容共用 `workspace` 表面；只有主内容一侧保留一条 `line/60` 细线。侧栏与主内容在窗口顶端连续。

### 2. 拖动、双击与窗口命令

- 顶部工作带使用 `data-tauri-drag-region="deep"`；按钮、链接、选择器等可交互元素按 Tauri 规则阻断拖动。
- Tauri 内置拖动脚本在双击时调用 `internal_toggle_maximize`。
- 窗口控制调用 `getCurrentWindow()` 的 `minimize`、`toggleMaximize`、`close`、`isMaximized`。
- capabilities 最小新增：
  - `core:window:allow-start-dragging`
  - `core:window:allow-internal-toggle-maximize`
  - `core:window:allow-minimize`
  - `core:window:allow-toggle-maximize`
  - `core:window:allow-close`
  - `core:window:allow-is-maximized`
  - 保留 `core:window:allow-set-theme`

### 3. 原生 Snap Layouts 子窗口

新增 `apps/desktop/src-tauri/src/window_chrome.rs`：

- 在 Tauri 主窗口上以 `SetWindowSubclass` 监听 `WM_SIZE`、`WM_DPICHANGED`、`WM_CLOSE`。
- 在最大化按钮位置创建 `XushuSnapOverlay` 子窗口：
  - `WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS`；
  - 背景为空，不绘制；
  - `WM_NCHITTEST` 返回 `HTMAXBUTTON`；
  - `WM_NCLBUTTONDOWN` 激活父窗口并切换最大化/还原；
  - `WM_NCMOUSEMOVE` / `WM_NCMOUSELEAVE` 向前端发出悬停事件。
- 覆盖层尺寸按 `GetDpiForWindow` 缩放；右侧偏移等于关闭按钮宽度。
- 覆盖层只占最大化按钮区域，不承载业务逻辑；前端仍拥有可见按钮与键盘可访问性。
- 使用 `DwmSetWindowAttribute` 设置 `DWMWA_WINDOW_CORNER_PREFERENCE = DWMWCP_ROUND` 与 `DWMWA_BORDER_COLOR = DWMWA_COLOR_NONE`，避免无装饰窗口在深色主题下出现白色 1px 边框。

### 4. 不使用子 Agent、不引入插件依赖

- 本轮由当前主会话完成设计与实现。
- 未新增第三方 Tauri 插件；`windows-sys 0.61.2` 此前已由 `xushu-core` 锁定，本轮仅在 desktop crate 中启用窗口相关 feature，版本不变。
- 未启用 `withGlobalTauri`；前端使用打包的 `@tauri-apps/api`。

### 5. 页面结构

- 一级页面（工作台、项目、任务、验收中心、诊断、设置）不再显示 19px 页面大标题；页面名称由顶部工作带承载，页面内容用 `sr-only h1` 保留语义。
- 列表页把主操作移入上下文工具栏（项目导入、任务新建、验收导出），不再保留独立 `PageHeader`。
- 详情页保留 `PageHeader`，因为实体名称（项目名/任务标题）是内容级标题。
- 工作台首屏不再以“工作台”标题开场，而是依次呈现：状态信号条 → 当前焦点任务 → 最近验证 → 需要处理/最近任务。

## 备选方案与取舍

| 备选 | 未采用原因 |
|---|---|
| 继续保留原生标题栏（ADR-0005） | 无法解决用户明确指出的“标题栏一层、应用内容一层”的顶部断层。 |
| 无装饰窗口 + 自定义标题栏，但不恢复 Snap Layouts | 最大化按钮的 Snap 悬停菜单会退化，不满足窗口行为要求。 |
| 直接使用 `tauri-plugin-frame` / `decorum` | 需要额外插件依赖、注入自带 HTML 标题栏、`withGlobalTauri` 或额外权限；与本项目已有工作带和最小权限目标重叠。 |
| 只在主窗口处理 `WM_NCHITTEST` | WebView2 子窗口先接收命中测试，Windows 不会调用主窗口的命中测试。 |
| 复制完整第三方插件 | 保留最小实现，避免引入不需要的标题栏样式、事件与配置面。 |

## 后果

**正面：**

- 窗口顶端成为 XUSHU 自己的工作带，侧栏、项目上下文、页面名称与窗口控制处于同一表面。
- 品牌区、项目选择、Git 状态、页面上下文与窗口控制可在 500–1920px 宽度内稳定布局。
- Windows 11 Snap Layouts 通过原生 `HTMAXBUTTON` 覆盖层恢复，不模拟按键或鼠标输入。
- 保留拖动、双击、最小化、最大化、还原、关闭与边缘缩放；Tauri 仍负责窗口命令。
- 未启用 `withGlobalTauri`，未新增第三方插件，权限面保持窄范围。
- `minWidth: 500` 满足 Microsoft 对 Snap Layouts “至少支持 500 effective pixels”的建议，并支持 1366 屏幕半屏贴靠。

**负面 / 风险：**

- 自绘标题栏必须持续维护原生行为回归；任何 Tauri/winit/Windows 变更都可能影响 Snap 覆盖层。
- Snap Layouts 的实际悬停菜单是 Windows 系统 UI，自动化只能验证 `HTMAXBUTTON` 命中，最终视觉需人工确认。
- 覆盖层是原生子窗口，无法通过 DOM/ARIA 直接访问；可见的最大化按钮仍由 React 提供键盘与辅助功能。
- 无装饰窗口的边缘缩放由 Tauri 的不可见缩放手柄承担，无法像原生 caption 一样在所有 DPI 组合下无条件保证。
- 多显示器、125%/175%/200% 缩放、macOS/Linux 未在本轮验证。
- 自定义标题栏意味着窗口外观完全由 XUSHU 负责，原生系统主题切换不会自动改变工作带颜色；XUSHU 主题系统负责。
- 前端 bundle 增加约 20 KB（窗口控制与事件订阅），仍为单 chunk。

## 验证要求

自定义标题栏相关改动至少需要：

1. `pnpm typecheck / lint / test / build / format`；
2. `cargo check --workspace --all-targets --locked`、`cargo fmt --check`、`cargo clippy -- -D warnings`、`cargo test --workspace`；
3. 真实窗口：拖动、双击最大化/还原、最小化、最大化、还原、关闭；
4. Snap 覆盖层：存在、位置随 DPI/尺寸更新、`WM_NCHITTEST` 返回 `HTMAXBUTTON`；
5. 1366×768、1600×900、1920×1080 与 500×480 窄窗口无页面级横向溢出；
6. 深浅主题、100%（CDP 视口仿真）与系统 150% DPI；
7. 人工视觉确认 Snap 悬停菜单、窗口圆角、焦点与多显示器行为。

## 回滚

若自定义标题栏在真实使用中出现不可接受的退化：

1. 将 `decorations` 改回 `true`，移除 `window_chrome` 安装与 `WindowControls` 的渲染；
2. 恢复 ADR-0005 的“原生标题栏 + 侧栏/PageHeader 一体化”方案；
3. 保留 `data-tauri-drag-region` 不生效时的普通页面布局；
4. 移除新增窗口权限（`start-dragging`、`internal-toggle-maximize`、`minimize`、`toggle-maximize`、`close`、`is-maximized`），保留 `set-theme`；
5. 不删除用户数据，不改变 Phase 1–3 的业务语义。

## 参考

- Tauri 2 Window Customization：<https://v2.tauri.app/learn/window-customization/>
- Tauri 2 WindowConfig：<https://v2.tauri.app/reference/config/>
- Tauri Snap Layouts issue #4531：<https://github.com/tauri-apps/tauri/issues/4531>
- Microsoft：Support snap layouts for desktop apps on Windows 11：<https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-snap-layout-menu>
- WM_NCHITTEST：<https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-nchittest>
- `tauri-plugin-frame`：<https://crates.io/crates/tauri-plugin-frame>
- 本地实现：`apps/desktop/src-tauri/src/window_chrome.rs`、`apps/desktop/src/app/workspace-chrome.tsx`、`apps/desktop/src/components/window-controls.tsx`