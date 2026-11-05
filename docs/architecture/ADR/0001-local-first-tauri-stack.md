# ADR-0001：采用 Tauri 2 + React + Rust/SQLite 的本地优先架构

- 状态：Accepted
- 日期：2026-11-05
- 决策者：项目负责人（用户）与主开发助手
- 相关文档：`MASTER_DEVELOPMENT_PLAN.md`、`docs/architecture/DEPENDENCIES.md`

## 背景

XUSHU V1 需要在 Windows 11 x64 上提供一个本地优先的桌面工作控制系统：

- 定义任务与验收标准；
- 收集用户明确授权的执行事件；
- 由应用独立运行验证命令并保存证据；
- 在工作区/任务变化后使旧证据过期；
- 关闭应用后仍能恢复项目与任务状态。

V1 不依赖云账户、远程数据库或后台微服务；AI 文本不能作为完成证据。

## 决策

采用以下冻结架构：

1. **桌面壳：Tauri 2.x。** 使用系统 WebView（Windows 上为 WebView2），Rust 作为可信后端。
2. **前端：React + TypeScript strict + Vite。** UI 使用 Tailwind CSS、shadcn/ui、Lucide；路由 React Router；异步状态 TanStack Query；纯 UI 状态 Zustand；表单 React Hook Form + Zod。
3. **核心服务：Rust。** 所有权限、路径校验、状态转换、SQLite 写入、验证命令执行都在 Rust 侧；前端仅通过受限 Tauri Commands 访问。
4. **数据库：SQLite + rusqlite + 版本化迁移。** 使用 UUID、UTC RFC3339、事务与 WAL；不向前端暴露通用 SQL。
5. **Git：** 以可执行文件 + 参数数组调用本机 `git`，V1 默认只读。
6. **Claude Code 接入：** 项目级 Hooks 必须由用户显式启用；`xushu-hook` 只写入白名单元数据；失败不阻塞 Claude Code。
7. **版本基线：** Tauri 2.12.x；Rust 1.90+（当前 stable 1.99.1）；Node 22.12+（本机 24.20.0）；TypeScript 6.0.x，暂不使用 7.0.x（无编译器 API，typescript-eslint 不兼容）。

## 备选方案与取舍

| 备选 | 未采用原因 |
|---|---|
| Electron | 安装包与内存开销更大；Node 运行时与渲染进程的权限边界更复杂；不符合最小权限目标。 |
| 云 SaaS / 本地 HTTP 服务 | V1 要求离线可用、本地数据优先；引入账户、同步与服务端安全面超出范围。 |
| 前端直接访问 SQLite | 会绕开任务状态机、验收门禁与路径授权；不可接受。 |
| 全量采集 Claude Code transcript | 违反最小采集与隐私边界；事件只作为“观察到某操作”的来源，不作为完成证据。 |
| TypeScript 7.0 | 原生编译器 7.0 不提供编译器 API，typescript-eslint 8.x 不支持；会导致 lint 链路不可用。 |

## 后果

**正面：**

- 本地优先、离线可用，敏感原文默认不出本机。
- Rust 侧形成权限与状态的可信边界，前端无法绕过验收门禁。
- Tauri 2 的 capabilities 模型支持最小权限配置。
- SQLite 单文件便于一致性备份与恢复演练。

**负面 / 风险：**

- Windows 开发必须安装 Rust MSVC 工具链与 C++ Build Tools；当前环境缺失，Phase 0 因此阻塞。
- Rust + Tauri 的学习与编译成本高于纯 Web 项目。
- Tauri 2.12 不再支持 Windows 7；本项目不以 Windows 7 为目标。
- 跨平台路径、进程终止与文件权限需要平台抽象；Windows 通过不代表 macOS 已验证。

## 回滚

V1 内不计划更换技术栈。若未来必须更换，需新增 ADR，说明原因、迁移成本、数据兼容性与回滚方案；不得直接推翻本决策。

## 参考

- <https://v2.tauri.app/start/prerequisites/>
- <https://v2.tauri.app/security/capabilities/>
- <https://v2.tauri.app/start/project-structure/>
- <https://vite.dev/guide/>
- <https://github.com/typescript-eslint/typescript-eslint/issues/12518>

## 修订记录

- **2026-11-05（第二轮 Phase 0）：** 用户已安装 Rust 1.99.1、MSVC Build Tools 2026、Windows SDK 10.0.26100 与 pnpm 12.16.1，并已实测可编译、链接与启动。Tauri 2.12.5 + React 19.3.1 + Vite 8.3.6 + TypeScript 6.0.3 的组合已通过本轮全部质量门禁并实际启动桌面窗口。原“当前环境缺失，Phase 0 因此阻塞”仅属于首轮历史事实，现已由 `docs/phases/PHASE_0_REPORT.md` 的第二轮结果取代；本 ADR 的架构决策不变。Node 下限按 jsdom 31 的实际要求记为 `^22.22.2 || ^24.15.0`（本机 24.20.0）。