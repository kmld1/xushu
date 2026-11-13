# XUSHU Phase 6.1 独立安全审计与灾难恢复验收

2026-11-14 · Windows 11 x64 · 基线 `06899e9986ee6983a623097235429c53fd0518f8` · `main`

**结论：普通 Windows PowerShell 7.6.9 的 Rust 门禁为 PASS，四项 Junction 安全用例为 `PRODUCT_PASS`。具备建立 Phase 6 本地 `validation pending` Git 检查点的条件。** 核心备份、正式切换、启动回退和回退失败保护已有隔离证据。Phase 6 整体验收仍为 PENDING，WebView2 交互、人工视觉、剩余故障注入、生产安装包和 macOS 实机继续待验收。

## 1. 独立验收矩阵

| 验收项 | 本轮状态 | 独立证据 |
| --- | --- | --- |
| 普通 Windows Rust 门禁 | PASS | 复核 17 个结果段及 203 条通过用例，0 失败、0 忽略。终端执行者记录退出码 0 |
| Windows Junction 产品断言 | `PRODUCT_PASS` | 普通终端的四个安全用例实际创建 Junction 并通过。AppContainer 的阻断记录见第 6 节 |
| `BACKUP_VERIFIED` | PASS | D 盘 WAL 源库、并发双行事务、验证失败和取消、同父目录发布、真实业务数据往返 |
| `MANIFEST_SECURITY_VERIFIED` | PARTIAL | 格式、散列、大小、实际 Schema、损坏库与外键断言通过。链接夹具和完整文件系统对抗范围见第 6 节 |
| `RESTORE_PRECHECK_VERIFIED` | PASS | 错误输入无法获取可用计划。过期、当前库变化、备份变化、并发和重放均受 Rust 门禁约束 |
| `ISOLATED_RESTORE_VERIFIED` | PASS | v6 隔离迁移成功。损坏库、外键违规、未来 Schema、声明不符和额外 Schema 对象被拒绝 |
| `RESTORE_SWITCH_VERIFIED` | PASS | Tauri 测试运行时调用真实命令处理器执行正式切换。核心测试验证业务行、关系和恢复后重新打开 |
| `CRASH_RECOVERY_VERIFIED` | PARTIAL | 四个边界的测试进程实际强杀并重启。半写入场景另由真实 Tauri 可执行程序启动回退。原生 Tauri 进程内故障注入和 Windows 重启为 NOT_RUN |
| `ROLLBACK_VERIFIED` | PASS | 最终校验失败恢复原值。无可用回退副本时保留现场，返回 `rollback_failed`，重启返回 `needs_manual` |
| `PRIVACY_VERIFIED` | PARTIAL | 备份范围、Spool 和 Salt 隔离、未加密提醒已核实。恢复后的真实 Claude Code 联调为 NOT_RUN |
| `DATA_ISOLATION_VERIFIED` | PARTIAL | 独立审计期间正常 DB、WAL、SHM 前后相等。D 盘隔离验证通过。普通终端 Junction 四项通过，其他链接变体待验收 |
| `UI_INTERACTION_VERIFIED` | ENVIRONMENT_BLOCKED | 真实 Tauri 和 WebView2 profile 已启动。Vite/CDP 回环被拒绝，真实 WebView2 交互断言为 NOT_RUN |
| `VISUAL_REVIEW_PENDING` | PENDING | 用户手动验收 |
| `PRODUCTION_INSTALLER` | NOT_RUN | 安装包、安装版恢复及 sidecar 分发待验收 |
| `MACOS_COMPATIBILITY_REVIEW` | PASS（源码预审） | Apple Silicon 构建、实际运行及安装包均为 NOT_RUN |

## 2. 数据资产、威胁模型与备份一致性

审查了项目约束、总开发计划、Phase 6 报告、ADR-0009、数据保护规范、设计系统、UI 审计、迁移 7、Rust 实现、Tauri IPC 和前端数据保护模块。迁移 7 增加备份登记、策略和恢复记录表。

- 备份资产包括项目、任务、验收、验证证据、Claude 元数据、记忆修订、决策关系、恢复快照与备份管理数据。数据库正文可能包含本地项目路径、验证输出和用户录入的敏感信息。
- 测试对 WAL 源库执行 SQLite Online Backup API，独立连接在单事务中写入成对记录。快照保留至少一个完整事务，逐事务检查两条记录，未发现跨事务拼接。快照整理为单个 SQLite 文件，关闭连接后按清单验证。
- 创建时先写目标目录下的 `.partial-*`，再在同一父目录改名发布。试验使用 D 盘同一目录的真实文件操作。校验失败或取消不登记为有效备份。恢复入口拒绝尚未发布的目录。Windows 备份文件独占打开时，预检拒绝，句柄释放后重试成功。
- 恢复业务样本包含两条记忆修订、确认后的取代关系、Claude 会话/事件、验收条件、验证证据及恢复快照。备份后新增任务从恢复后的快照消失。原任务仍为 `awaiting_verification`，备份前后均不能绕过人工验收进入 `done`。
- 在线备份的事务一致性参考 [SQLite Online Backup API](https://sqlite.org/backup.html)。同父目录改名和不同挂载点限制参考 [Rust rename](https://doc.rust-lang.org/std/fs/fn.rename.html)。实际断电、跨卷、持续 30 秒外部独占、真正磁盘耗尽为 NOT_RUN。容量规则使用模拟可用字节数执行。

## 3. Manifest 与恶意输入

| 测试 | 结果 |
| --- | --- |
| 缺字段、未知字段、非法 JSON、重复文件、绝对路径、相对穿越 | PASS，清单校验拒绝 |
| 非法格式版本、未来 Schema、清单与实际 Schema 不符 | PASS，拒绝授权 |
| 清单字节变化、SHA-256 不匹配、大小不符、截断或损坏 | PASS，预检失败或已签发 token 失效 |
| 声明超过 2 GiB、缓冲读取越界 | PASS，真实大于 2 GiB 的文件为 NOT_RUN |
| 重新计算散列后的损坏库、外键损坏 | PASS，隔离 SQLite 或关系检查失败 |
| 非预期 SQLite Schema 对象 | PASS，按项目迁移生成预期 Schema 并逐项比较，拒绝额外对象 |
| Junction、符号链接、重解析点现场 | Junction 四项 `PRODUCT_PASS`。符号链接及其他重解析点产品断言为 NOT_RUN |

清单限 256 KiB，数据库限 2 GiB，哈希与复制采用 64 KiB 分块。隔离 SQLite 校验使用约 60 秒查询进度检查，并启用 defensive 模式、关闭 trusted schema。该时限在 SQLite 回调触发时检查，文件 I/O 与系统调用不受它硬中断。用真实大文件打满限额及触发 60 秒中止均为 NOT_RUN。参考 [SQLite 不可信数据库防护](https://www.sqlite.org/security.html) 和 [SQLite 查询进度回调](https://www.sqlite.org/c3ref/progress_handler.html)。

格式合法、散列匹配、SQLite 页完整、外键无违规和业务归属正确是不同证据。SHA-256 可随替换后的内容重新计算。来源真实性与保密性没有通过该机制认证。

## 4. 正式恢复、中断和回退

| 场景 | 结果 | 证据 |
| --- | --- | --- |
| A · 预检失败 | PASS | 损坏、占用等预检失败，未写活动日志，隔离当前数据保留 |
| B · 隔离恢复失败 | PASS | 损坏库、外键损坏、未来 Schema 与不一致 Schema 均未产生可切换计划 |
| C · 回退副本已创建 | PASS（应用状态子进程） | 在 `prepared` 实际强杀，第二进程得到 `abandoned`，当前值保留 |
| D · 正式替换途中 | PASS（受控边界及真实应用启动） | 构造已移动原库及半写入当前库，强杀夹具。隔离 Tauri 可执行程序启动后独立 SQLite 查询得到 `integrity_check=ok`、外键违规 0、原值 17、历史 `rolled_back/used` |
| D · 文件切换完成但未提交最终结果 | PASS（应用状态子进程） | 实际执行 `switch_files` 后强杀，第二进程自动回退 |
| E · 最终校验失败 | PASS（受控注入） | 切换后故意损坏隔离当前库，最终校验返回错误。桌面状态失败处理回退并写入历史 |
| F · 回退也失败 | PASS（受控注入） | 回退快照暂移，返回 `rollback_failed` 并保留当前文件、原件、日志和隔离副本。重启返回 `needs_manual`，不开放数据库 |
| G · 恢复后重新打开 | PASS（核心与实际启动） | 核心完整业务样本重新打开并查询。真实 Tauri 启动的半写入回退另用 SQLite 查询复核 |

测试进程强杀是实际 OS 进程终止。半写入内容由测试夹具在指定边界构造。原生 Tauri 中通过 WebView IPC 发起恢复、精确杀同一 Tauri 进程及物理重启 Windows 为 NOT_RUN。

## 5. 正式恢复授权

- Tauri MockRuntime 调用真实 `preflight_restore`、`perform_restore` 命令处理器。无 token、未勾选、错误确认文字、清单字节变化、失败后重放和成功后重放均被拒绝。测得 token 期限约 900 秒。用受控时钟值复核了过期拒绝。
- 当前数据库变化时要求重新预检。记录 SQLite 本连接累计变化与其他连接 `data_version`，在持有 AppState 数据库锁后和关闭当前库前再次核对。并发提交两个有效 token 仅一笔进入切换。另一桌面实例也不能同时开放同一数据目录。
- 仍受同用户外部 SQLite 写进程、同步软件和底层文件系统语义影响。Windows 句柄保护和应用进程锁覆盖的对象不同。验证资源上限和来源认证仍需按第 3 节理解。

## 6. Windows 文件系统、Hooks 与隐私

- 实测 UNC、设备路径、父目录穿越拒绝。已锁定 Windows 父目录不可改名，备份文件独占时不可预检。Codex AppContainer 历史运行中，Junction 夹具创建为 `ENVIRONMENT_BLOCKED`。三项 Claude 用例因夹具失败退出，备份 Junction 普通模式提前返回，严格模式退出 101。历史运行的四项产品断言均为 NOT_RUN。
- AppContainer 符号链接探针要求管理员权限，硬链接探针被拒绝，两者的产品断言均为 NOT_RUN。其他重解析点、只读文件、跨卷、恶意竞态矩阵和系统重启为 NOT_RUN。
- SQLite 快照没有将文件形式的 Spool、Salt、Claude 配置、WebView 数据或导出目录纳入。Spool 和 Salt 在真实恢复文件切换前后字节不变，隔离项目没有生成 `.claude`。旧事件导入由项目 ID、配置代次和 Salt 绑定的项目指纹限制。其他项目、接收器变更和无 Salt 时接收失败在既有测试中重新执行。完整的“积压 Spool + 恢复 + 真实 Claude Code”场景为 NOT_RUN。
- 源码检查显示白名单收集事件元数据，原始 Prompt/Transcript 未被收集，项目状态使用项目级设置。正常用户级 Claude Code 配置与真实凭据未读取。备份目录清单用固定相对路径。数据库中业务路径按快照范围保留。
- **保密性风险：中高。** 在单机受控账户及已有系统卷加密条件下可接受为 V1 的本地备份边界。共享磁盘、第三方云同步、支持工单附件或便携介质属于高风险。若 V1 要承诺这些场景，需在承诺前提供带可恢复密钥管理的加密能力。当前 UI、Manifest 与文档均明确标注未加密，审计未新增加密功能。
- 恢复会把数据库内历史登记还原到备份时点。恢复源的登记在创建备份快照之后写入，因此恢复源可能需要通过文件夹重新选择。恢复操作历史由新库/回退库及磁盘日志补记。回退文件缺失时日志和证据保留，数据库停止提供服务。

普通 Windows PowerShell 7.6.9 复测使用下列四个用例。日志第 96 行记录 Windows 创建联接成功，第 100 行记录备份用例通过。其余三项由测试夹具强制验证联接创建和实际重定向目标，随后才执行产品断言。

| 用例 | 状态 | 覆盖范围 |
| --- | --- | --- |
| `junction_backup_target_is_refused_when_environment_allows_creation` | `PRODUCT_PASS` | 备份目标目录与其祖先为 Junction 时均拒绝 |
| `closeout_registered_root_rejects_junction_even_after_new_preview` | `PRODUCT_PASS` | 已登记项目根目录被 Junction 替换时，原预览授权和重新预览均无法继续安装 |
| `closeout_rejects_redirected_parent_and_backup_directory` | `PRODUCT_PASS` | 拒绝被重定向的项目父目录和 Hook 备份目录，保留原配置 |
| `directory_redirection_is_refused` | `PRODUCT_PASS` | 拒绝被重定向的 `.claude` 目录，目标中未生成配置 |

## 7. macOS Apple Silicon V1 预审

| 项目 | 风险及验证要求 |
| --- | --- |
| 路径规范化 | APFS 大小写、Unicode 与 `/var`、`/tmp` 别名可能触发严格 canonical 路径比较。实际 AppData、用户选择目录及沙箱目录须实测 |
| 进程锁 | Rust `File::try_lock` 在 Unix 是协作锁，在 Windows 使用文件锁接口。Unix 路径替换、锁文件删除和外部 SQLite 写入需要独立测试。参考 [Rust File::try_lock](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock) |
| 原子改名与耐久性 | Unix 同挂载点改名原子，但需验证父目录同步、系统断电、应用杀进程和重启顺序。现阶段 Windows 测的是进程异常 |
| 文件权限和链接 | Unix 新建文件用 0600；目录权限继承 umask。Unix 目录检查与路径操作之间缺少 Windows 式拒绝删除句柄保护。检查 symlink、硬链接、移除/替换父目录与可移动卷 |
| 空间和数据库切换 | Unix 空闲空间查询当前返回未知，容量预检可能放行。实测 APFS 容量不足、外部连接、WAL/SHM、原子改名和崩溃后日志 |
| 应用数据与 Hook sidecar | Tauri 平台数据目录；需实测 `.app/Contents/MacOS`、签名 sidecar、迁移路径和失配提示。Windows 测试工具链未证明 arm64 构建 |

## 8. 已复现并修复

| 严重度 | 复现条件 | 修复和回归证据 |
| --- | --- | --- |
| 高 | 损坏活动恢复日志且当前库缺失 | 原启动会创建空库。修复后 `needs_manual` 阻断数据库初始化及清理，保留现场 |
| 高 | 已签发预检后改动当前数据库 | 原 token 仍能覆盖新数据。新变更戳阻断，旧 token 不可重用 |
| 高 | 回退副本不存在 | 原逻辑先移走当前库。新逻辑先校验回退源，保留原文件，无法回退时隔离数据库 |
| 高 | 两个应用实例打开相同数据目录 | 原来两个实例都打开。新增 OS 文件锁，第二个实例失败，首实例退出后可重新打开 |
| 中高 | 手动将带完整清单的 `.partial-*` 目录送入预检 | 已拒绝恢复未发布目录 |
| 中高 | 当前支持的两个 Schema 版本之间故意错报清单版本，或增加额外 SQLite Schema 对象 | 已拒绝清单/文件版本不一致及未知 Schema |
| 中 | 外键坏库整体校验失败，但“SQLite 与业务关系”汇总行显示通过 | 汇总状态现在与真实关系校验一致 |
| 测试夹具 | 旧用例使用 `TempDir` 隐式删除 Git 夹具，Windows 静默删除失败 | 改为显式移动隔离仓库并验证原路径消失。产品断言保留 |

另外检查了 checkpoint 返回的 busy 状态、终态日志持久化、启动补记恢复历史、备份路径与隔离目录持有期。测试运行时在 MSVC 下需 Common Controls v6，构建已用同一依赖清单支持应用与测试入口。

## 9. 质量命令与执行记录

### 普通 Windows PowerShell 7.6.9 复测归档

复核 `target/phase6-local-review/cargo-test.log` 的 17 个 `test result:` 结果段和逐条用例。12 个有用例的结果段合计 203 PASS，0 FAIL，0 IGNORED，0 filtered out。203 是测试运行器统计，包含桌面子进程入口用例。另有 5 个零用例结果段。日志正常到达文档测试末尾。Rust 增量缓存硬链接失败后回退为复制，构建及测试继续完成。

| 命令 | 退出码 | 结果 |
| --- | --- | --- |
| `cargo test --workspace --locked` | 0（终端执行者记录） | PASS，203 通过、0 失败、0 忽略、四项 Junction `PRODUCT_PASS` |

日志本体未写入进程退出码字段。0 取自执行者的终端记录，和完整成功日志一致。日志 SHA-256 为 `EDB73178ACE243A44D31DF4FFC413E4124B3BD63E620F5A0CF205F7B9A3AF1CD`。此轮归档只复核现有终端证据，保留原始日志。Phase 6 原报告记录的 186 项 Rust 与桌面 E2E 是此前实施轮次的结果。

### Codex AppContainer 独立审计历史

| 命令 | 退出码 | 结果 |
| --- | --- | --- |
| `pnpm typecheck` | 0 | PASS |
| `pnpm lint` | 0 | PASS |
| `pnpm test` | 0 | 83 项通过，11 个文件 |
| `pnpm build` | 0 | PASS，保留既有 JS chunk 大小提示 |
| `pnpm format` | 0 | PASS |
| `cargo fmt --all -- --check` | 0 | PASS |
| `cargo check --workspace --all-targets --locked` | 0 | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | PASS |
| `cargo test --workspace --locked` | 101 | 停在三项 Junction 夹具创建失败 |
| `cargo test --workspace --locked --no-fail-fast -- --nocapture` | 101 | Harness 200 通过、3 失败。实际顶层断言通过 198 项，另有 1 项备份 Junction 未执行及 1 个子进程入口 |
| `cargo test -p xushu-core --test backup --locked -- --nocapture` | 0 | 22 项 harness 通过。21 项真实断言通过，1 项 Junction 环境阻断 |
| 备份 Junction 单项复测，`XUSHU_REQUIRE_JUNCTION_TEST=1` | 101 | 无法创建真实夹具时严格失败。产品断言为 NOT_RUN |
| `cargo test -p xushu-desktop --lib --locked -- --nocapture` | 0 | 10 项 harness 通过，其中 1 项是子进程入口，实际执行由父用例触发 |
| `python scripts/desktop-e2e.py` | 1 | Windows 回环 WinError 10013，真实 WebView2 交互 NOT_RUN |
| 嵌入式隔离 Tauri 开发构建与原生进程重启验证 | 0 | 成功创建隔离 DB 和 WebView profile，恢复后数据库与历史断言通过 |
| `git diff --check` | 0 | PASS |

该历史运行的失败项为 `closeout_registered_root_rejects_junction_even_after_new_preview`、`closeout_rejects_redirected_parent_and_backup_directory`、`directory_redirection_is_refused`。三项在 Junction 夹具创建时退出，该次产品断言为 NOT_RUN。普通终端的独立结果见本节上方。

首轮 Corepack 尝试联网拉包并返回 1，指向已缓存的固定 pnpm 12.16.1 后重新执行，未下载依赖。Rust 验证过程中还遇到 MSVC 重复资源、缺少 Common Controls v6 清单和一次并行执行时的链接目标占用。清单已修正，重新链接通过。隔离 TEMP 位于工作区 `target` 下，Git fixture 需要进程级 `GIT_CEILING_DIRECTORIES` 防止非仓库用例向上识别 XUSHU 仓库。

证据目录为 `target/phase6-1-f7f4261b7f1a`。主要日志为 `workspace-final.log`、`desktop-verified.log`、`native-crash-reopen-evidence.txt`、`pnpm-*-final.log` 和 `normal-before.json` / `normal-after.json`。

## 10. 正常数据库指纹与 Git

下表为原 Phase 6.1 独立审计在正式应用数据目录记录的只读指纹。该轮前后大小、SHA-256 和 UTC 修改时间逐项相同，也与已记录的 Phase 6 历史散列相同。普通终端复测日志未记录正常库指纹。

| 文件 | 字节 | 前后 SHA-256 |
| --- | ---: | --- |
| `xushu.db` | 4096 | `89A2318C76452BE80E1D743F163F3DD96623B2B05604AA06985C6B5501238362` |
| `xushu.db-wal` | 2463792 | `096E2C713500F8BC3F31496A30197033816DB4BC12C350A2459E5265186A9871` |
| `xushu.db-shm` | 32768 | `ADD4EBABE42A90EA21E3572567FFB8FEA98AB732CD84F11DB0C2E1D4F351310A` |

Git 保持原 HEAD，暂存区为空。工作区有 29 个已跟踪文件修改，28 个未跟踪文件（按默认 `git status --short` 折叠为 12 个条目）。`docs/phases/PHASE_4_REAL_HOOKS_REGRESSION.md` 仍未跟踪，前后 SHA-256 为 `AC5C24BE5202166E83EA5AB9E75CF27C3A5D7E5B2E4C4ADD353C28BDEC36CA1F`。

复核了所有已跟踪差异与非忽略的新文件。以下 56 个文件属于 Phase 6 检查点候选。候选均为文本，常见凭据、私钥、认证 URL 和明文密码模式扫描无命中。Git 忽略运行数据库、备份、Spool、Salt、用户级本地配置和 `target/` 原始证据。独立 Phase 4 Hooks 回归报告已排除。本轮唯一写入为本报告。

文档保留本地工程路径和历史数据库指纹，用于本地审计。原始测试日志另含本机临时目录路径，继续放在忽略目录中。公开发布前需要单独复核文档中的环境细节。

```text
apps/desktop/src-tauri/build.rs
apps/desktop/src-tauri/Cargo.toml
apps/desktop/src-tauri/src/commands.rs
apps/desktop/src-tauri/src/data_protection_commands.rs
apps/desktop/src-tauri/src/error.rs
apps/desktop/src-tauri/src/lib.rs
apps/desktop/src-tauri/src/state.rs
apps/desktop/src/App.test.tsx
apps/desktop/src/App.tsx
apps/desktop/src/app/app-layout.tsx
apps/desktop/src/app/workspace-chrome.tsx
apps/desktop/src/features/data-protection/api.ts
apps/desktop/src/features/data-protection/backup-policy-dialog.tsx
apps/desktop/src/features/data-protection/backup-verify-sheet.tsx
apps/desktop/src/features/data-protection/create-backup-dialog.tsx
apps/desktop/src/features/data-protection/data-protection-badges.tsx
apps/desktop/src/features/data-protection/data-protection-display.test.ts
apps/desktop/src/features/data-protection/data-protection-display.ts
apps/desktop/src/features/data-protection/integrity-report.tsx
apps/desktop/src/features/data-protection/privacy-inventory.tsx
apps/desktop/src/features/data-protection/queries.ts
apps/desktop/src/features/data-protection/restore-wizard-sheet.tsx
apps/desktop/src/lib/domain.ts
apps/desktop/src/pages/data-protection-page.test.tsx
apps/desktop/src/pages/data-protection-page.tsx
Cargo.lock
CLAUDE.md
crates/xushu-core/Cargo.toml
crates/xushu-core/migrations/0007_backup_restore_privacy.sql
crates/xushu-core/src/backup/integrity.rs
crates/xushu-core/src/backup/manifest.rs
crates/xushu-core/src/backup/mod.rs
crates/xushu-core/src/backup/overview.rs
crates/xushu-core/src/backup/paths.rs
crates/xushu-core/src/backup/restore.rs
crates/xushu-core/src/backup/service.rs
crates/xushu-core/src/claude/mod.rs
crates/xushu-core/src/claude/path_guard.rs
crates/xushu-core/src/db/migrations.rs
crates/xushu-core/src/db/mod.rs
crates/xushu-core/src/error.rs
crates/xushu-core/src/lib.rs
crates/xushu-core/tests/backup.rs
crates/xushu-core/tests/memory.rs
crates/xushu-core/tests/persistence.rs
docs/architecture/ADR/0009-backup-restore-data-protection.md
docs/architecture/CLAUDE_HOOKS.md
docs/architecture/DATA_PROTECTION.md
docs/architecture/DEPENDENCIES.md
docs/design/DESIGN_SYSTEM.md
docs/design/UI_AUDIT.md
docs/phases/PHASE_6_1_INDEPENDENT_AUDIT.md
docs/phases/PHASE_6_REPORT.md
MASTER_DEVELOPMENT_PLAN.md
README.md
scripts/desktop-e2e.py
```

## 11. 普通 Windows PowerShell 7 复测

普通终端 Rust 与 Junction 本轮已通过。以下脚本保留供后续复测，并包含本轮尚未执行的 WebView2 交互验证。Junction 复测显式要求真实夹具，创建失败即报错。桌面测试使用独立应用 identifier 和 D 盘数据目录。

```powershell
Set-Location -LiteralPath 'D:\MyProject\xushu'
$auditDir = Join-Path (Join-Path (Get-Location).Path 'target') ('phase6-1-retest-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $auditDir -ErrorAction Stop | Out-Null
$env:TEMP = $auditDir
$env:TMP = $auditDir
$env:GIT_CEILING_DIRECTORIES = $auditDir
$env:XUSHU_REQUIRE_JUNCTION_TEST = '1'

cargo test -p xushu-core --test backup --locked -- --nocapture
$backupExit = $LASTEXITCODE
cargo test --workspace --locked --no-fail-fast -- --nocapture
$rustExit = $LASTEXITCODE

python scripts/desktop-e2e.py
$desktopExit = $LASTEXITCODE

$history = Join-Path (Get-Location).Path 'target/phase6-audit'
$restoreScenario = Join-Path $history 'phase6-e2e.mjs'
$layoutScenario = Join-Path $history 'phase6-layout.mjs'
if ((Test-Path -LiteralPath $restoreScenario) -and (Test-Path -LiteralPath $layoutScenario)) {
    $newRestore = Join-Path $auditDir 'phase6-e2e.mjs'
    $newLayout = Join-Path $auditDir 'phase6-layout.mjs'
    foreach ($scenario in @(@($restoreScenario, $newRestore), @($layoutScenario, $newLayout))) {
        $source = Get-Content -Raw -LiteralPath $scenario[0] -Encoding utf8
        $source.Replace('"phase6-audit"', ('"' + (Split-Path -Leaf $auditDir) + '"')) |
            Set-Content -LiteralPath $scenario[1] -Encoding utf8
    }
    python scripts/desktop-e2e.py --scenario $newRestore
    $restoreExit = $LASTEXITCODE
    if ($restoreExit -eq 0) {
        $snapshot = Join-Path $auditDir 'phase6-restored.db'
        python scripts/desktop-e2e.py --snapshot $snapshot --scenario $newRestore
        $reopenExit = $LASTEXITCODE
    }
    python scripts/desktop-e2e.py --scenario $newLayout
    $layoutExit = $LASTEXITCODE
}
[pscustomobject]@{
    Backup = $backupExit
    Rust = $rustExit
    Desktop = $desktopExit
    Restore = $restoreExit
    Reopen = $reopenExit
    Layout = $layoutExit
    Evidence = $auditDir
}
```

## 12. 手动视觉与最终门禁

- [ ] 在 500×480、1366×768、1600×900、1920×1080 检查数据保护中心、恢复历史、资产清单与 Hooks 隐私说明。分别切换浅色、深色和跟随系统。
- [ ] 检查创建备份、备份策略、完整性校验和七步恢复向导。验证 Tab、Shift+Tab、Enter、Space、Esc、关闭后焦点恢复与确认文本。
- [ ] 验证 Dialog 内滚动、长路径、错误信息、页面横向溢出，以及恢复前后启动提示。使用用户手动提供的画面签收。
- [x] 普通 Windows 终端 Rust 全量测试及四项 Junction 产品断言。
- [ ] 普通 Windows 环境的独立 WebView2 交互复测，包含恢复、关闭重开和布局交互。
- [ ] 实际 Tauri 进程内的切换中断及回退失败、真实磁盘写满与权限拒绝、最终备份目标冲突、持续独占 30 秒、超过 2 GiB 的真实文件、跨卷、链接变体、Windows 系统重启和不同 Windows 账户仍为 NOT_RUN。受控子进程和 Rust 门禁证据保留原验收范围。
- [ ] 持续并发恢复与 token 压力测试、屏幕阅读器、真实 Windows 剪贴板、多显示器及 Snap 菜单仍为 NOT_RUN。
- [ ] 恢复后的真实 Claude Code Hooks 联调、生产安装包及 sidecar、macOS Apple Silicon 构建和实机验收均为 NOT_RUN。

**Phase 6 可建立本地 `validation pending` 检查点。** 候选路径见第 10 节，建议提交标题为 `chore(xushu): checkpoint phase 6 (validation pending)`。本轮保持暂存区和 Git 历史原状。检查点仅冻结当前工作与验证证据。Phase 6 完整验收仍为 PENDING，Phase 7 尚未启动。
