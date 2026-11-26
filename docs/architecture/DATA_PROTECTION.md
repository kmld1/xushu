# XUSHU 数据保护、备份格式与灾难恢复规范

> 状态：Phase 6 代码、自动化与隔离桌面 E2E 已实施；人工视觉与生产安装包仍为 PENDING。
> 决策：`docs/architecture/ADR/0009-backup-restore-data-protection.md`
> 报告：`docs/phases/PHASE_6_REPORT.md`

## 1. 数据资产清单

| 资产 | 存储位置 | 权威 | 可重建 | 敏感 | 进入备份 | 导出 | 保留/清理 | 丢失影响 |
|---|---|---|---|---|---|---|---|---|
| 项目与任务 | 应用数据目录 / `xushu.db` | 是 | 否 | 是（路径、用户文本） | 是 | 通过一致性备份 | 归档不删除历史 | 项目/任务上下文丢失 |
| 验收条件与状态历史 | `xushu.db` | 是 | 否 | 否 | 是 | 验收报告 Markdown | 随任务保留 | 完成门禁无法追溯 |
| 验证记录与证据 | `xushu.db` | 是 | 否 | 是（路径、脱敏输出摘要） | 是 | 验收报告 Markdown | 过期证据保留并标记 STALE | 无法证明验收成立 |
| Claude 集成本地状态 | `xushu.db` | 否（真实配置在项目中） | 是可重建状态 | 否 | 是 | 无独立导出 | 随项目保留 | 忘记期望启用状态 |
| 项目 `.claude/settings.local.json` | 项目仓库 | 是 | 可重建 | 是（可能含其他 Hooks） | 否 | 不导出 | 由项目/Claude Code 管理 | Hook 采集停止 |
| Claude 事件元数据 | `xushu.db` | 否（观察） | 否 | 是（伪名、工具名） | 是 | 活动 Sheet 查看 | 30 天 / 每项目 5000 事件 / 500 会话 | 失去观察历史 |
| Hook Spool | 应用数据目录 / `hook-inbox/pending` | 否 | 否 | 是（白名单元数据） | 否 | 不导出 | 导入后移除 | 未导入事件丢失 |
| 伪名化 Salt | 应用数据目录 / `hook-inbox` | 否 | 可重新生成 | 是（运行时秘密） | 否 | 不导出/不显示 | 随应用数据目录保留 | 新旧伪名无法关联 |
| 长期记忆与修订 | `xushu.db` | 否（上下文） | 否 | 是（用户录入） | 是 | 无独立导出 | 归档保留历史 | 失去开发上下文 |
| 开发决策与修订 | `xushu.db` | 否（上下文） | 否 | 否 | 是 | 无独立导出 | 追加式，不原地改写 | 失去决策依据 |
| 恢复快照 | `xushu.db` | 否 | 可由当前库重新生成 | 是（路径、记忆副本） | 是 | 可复制指令 | 每项目最近 20 份 | 历史快照不可恢复 |
| 备份文件 | 用户选择的本地目录 | 否 | 否 | 是（未加密数据库副本） | 否 | 备份文件本身可复制 | 自动备份按策略；手动不自动删除 | 无法恢复旧时点 |
| 验收报告导出 | 应用数据目录 / `exports` | 否 | 是 | 是（路径、脱敏输出） | 否 | 可复制 | 不自动清理 | 可重新导出 |
| 界面偏好（主题） | WebView2 本地存储 | 否 | 是 | 否 | 否 | 否 | 随 WebView 配置 | 主题重置 |
| 日志/诊断 | stderr（不持久化） | 否 | 是 | 否（仅错误码与计数） | 否 | 否 | 进程退出即丢失 | 失去本次诊断线索 |

## 2. 威胁模型

### 2.1 资产与信任边界

- 用户本人：可以查看、创建、校验、恢复、删除自己的数据；
- 本机其他进程：不可信，可能替换目录、占用文件、注入恶意备份或读取未加密备份；
- 备份文件：不可信输入；可能被截断、篡改、替换清单、包含未来 Schema 或路径穿越；
- 应用进程：可信边界；所有 SQL、路径、状态与确认在 Rust 侧校验；
- 前端 WebView：受限；没有通用 SQL/Shell/文件系统权限；
- Claude Code Hooks：可选观察源；事件不是完成证据；
- 崩溃/断电：可能在备份、切换、回退或删除的任意阶段中断。

### 2.2 威胁与缓解

| 威胁 | 影响 | 缓解 | 残余风险 |
|---|---|---|---|
| 运行时复制 `.db` | 备份缺少 WAL 或页面不一致 | Online Backup API + 快照完整性检查 | 极端磁盘错误仍可能损坏 |
| 并发写/任务编辑/Hook 导入/记忆更新 | 备份时点不一致 | 应用层数据库互斥 + SQLite 备份 API；Rust 并发写测试 | 外部进程绕过应用互斥，仍由 SQLite 锁与完整性检查兜底 |
| 备份中断/磁盘满/文件占用 | 留下可误认的成功备份 | `.partial-*` 临时目录；失败删除；清单最后写入；原子改名；登记失败回滚 | 删除失败时留下明确命名的 partial 目录，不登记、不显示为有效备份 |
| 恶意/损坏备份 | 恢复后数据损坏或目录逃逸 | 严格 JSON 白名单、单文件路径、未知字段/版本拒绝、SHA-256、隔离 SQLite 检查、容量上限 | 校验和不证明来源可信；只应恢复信任的备份 |
| 未来 Schema / 旧 Schema | 静默降级/丢表 | 未来版本拒绝；旧版本在隔离副本迁移并检查 | 极端旧版本缺少迁移路径时拒绝 |
| 覆盖当前数据库 | 当前变更丢失 | 只读预检、隔离验证、一次性 token、输入“恢复”+勾选、回退副本 | 用户确认后仍会丢失备份之后的数据 |
| 切换中断/崩溃 | 半替换数据库 | 恢复日志 + 启动回退；原库/WAL/SHM 移到回退目录 | 回退也可能失败，需人工检查 |
| 并发恢复/备份 | 文件竞争与状态错乱 | 单一备份/恢复互斥；恢复开始即作废旧预检；运行中验证阻断 | 崩溃恢复只能有一个进程 |
| Junction/符号链接/UNC | 写入或读取越界 | 句柄 + 重解析点校验；拒绝 UNC；文件不跟随链接；macOS 拒绝可写 ACL、远程卷与关闭所有权的卷；备份快照只在私有工作区交给 SQLite | 同 UID 且可写应用数据目录的进程或 root 仍可在连接存续期替换目录项；进程内路径检查不能建立同用户隔离 |
| Salt/Spool 泄露 | 会话关联风险 | 不进入备份、不导出、不显示 Salt；Spool 只含白名单元数据 | 本机可读取应用数据目录的进程仍可能读取 |
| 备份文件泄露 | 项目路径/记忆/证据内容泄露 | 明确未加密、UI 警告、只保存在受控位置、校验和提示 | Phase 6 无加密；获得文件者可以读取 |
| Hooks 被恢复流程修改 | 未经授权修改外部项目 | 恢复只替换数据库；不写项目 `.claude`；结果提示人工复核 | 用户仍需自行确认接入状态 |
| 自动清理误删手动备份 | 用户数据丢失 | 只清理 `automatic`、登记匹配、清单匹配、仅已知文件的备份 | 用户手动删除仍需确认 |
| 日志/错误泄露敏感信息 | 隐私泄露 | 错误码、固定文案、无路径/SQL/原文；备份 Manifest 无绝对路径 | 本地路径在本机 UI 显示供用户定位 |

## 3. 备份格式 v1

### 3.1 目录

```text
<用户目标目录>/
└── xushu-backup-20261113T130000Z-02d8fc58/
    ├── manifest.json
    └── xushu.db
```

创建过程：

```text
.user-chosen-target/.partial-<uuid>/
  ├── xushu.db          ← Online Backup API 快照
  └── manifest.json     ← 快照校验完成后写入
        │
        ├── 单文件整理 + integrity_check + 业务关系检查
        ├── 重新读取清单 + SHA-256 校验
        └── 同卷 rename → xushu-backup-<UTC>-<short-id>/
```

### 3.2 Manifest 示例

```json
{
  "formatVersion": 1,
  "product": "XUSHU",
  "backupId": "02d8fc58-8601-48e6-8603-e85e386c1760",
  "appVersion": "0.1.0",
  "schemaVersion": 7,
  "createdAt": "2026-11-13T13:01:14.000Z",
  "origin": "manual",
  "scope": "database",
  "encrypted": false,
  "complete": true,
  "files": [
    {
      "path": "xushu.db",
      "size": 413696,
      "sha256": "<64 位小写十六进制>"
    }
  ]
}
```

规则：

- `formatVersion` 只接受 1；未来版本明确拒绝；
- `product` 必须是 `XUSHU`；
- `backupId` 必须是 UUID；
- `schemaVersion` ≥ 1；高于当前应用支持的版本拒绝；低于当前版本在隔离副本升级；
- `origin` 为 `manual` 或 `automatic`；来源是标签，不是可信证明；
- `scope` 只接受 `database`；
- `encrypted` 必须为 `false`（v1 不支持加密）；`complete` 必须为 `true`；
- 只允许一个 `files` 项，路径必须正好是 `xushu.db`；
- 数据文件大小 1 B–2 GiB；清单 ≤ 256 KiB；
- 未知字段拒绝。

### 3.3 校验边界

- SHA-256 证明数据文件内容与清单一致；
- SQLite `integrity_check` 证明页面结构可读；
- 外键与业务关系检查证明项目归属、验证/证据、记忆/决策修订与取代关系一致；
- 这些检查都不证明备份来源可信、不证明任务验收成立，也不证明业务语义完全正确。

## 4. 一致性备份机制

1. 应用层 `backup_mutation` 互斥，备份/恢复/清理串行；数据库连接由互斥连接保护。
2. `Database::backup_to_with_cancel` 使用 `rusqlite::backup::Backup` 分页复制；遇到 Busy/Locked 重试，30 秒后返回明确错误；支持用户取消。
3. 快照写入用户目标目录内的 partial 目录，成功后用 `compact_for_backup` 合并 WAL 并切换到 DELETE 日志模式，使备份成为单文件。
4. 单文件通过 `DataIntegrityService`：SQLite 页面完整性、外键、Schema、项目/任务/验收/验证/证据/Claude/记忆/决策/快照归属与修订历史。
5. 计算 SHA-256，写清单，重新读取清单并再校验一次。
6. 同卷原子改名；登记到 `backup_records`。任何一步失败都不登记，且删除本应用创建的 partial/final 目录（尽力而为）。

## 5. 恢复状态机

```text
选择备份
  │
  ▼
preflight（只读）
  ├── 清单/版本/路径/容量/SHA-256
  └── 隔离副本：复制、迁移、完整性、业务关系
        │
        ├── 失败 → ready=false，无 token，不进入恢复
        ▼
确认（token + 输入“恢复” + 勾选）
        │
        ▼
执行
  ├── 重新校验备份与隔离副本
  ├── 检查正式恢复容量
  ├── 拒绝运行中的验证
  ├── 作废其它预检
  ├── 创建当前数据库回退副本
  ├── checkpoint + 关闭连接
  ├── 写 journal（prepared → switching）
  ├── 移动当前 DB/WAL/SHM → 回退目录
  ├── 复制隔离副本 → 正式路径 + SHA-256
  ├── 重新打开 + 最终完整性/业务关系检查
  │     ├── 通过 → 记录 completed → 归档 journal
  │     └── 失败 → 自动回退 → 记录 rolled_back / rollback_failed
  └── 清理本次隔离工作区（保留回退目录）
```

预检与确认阶段显示备份之后当前数据库的时间戳变更计数（任务、项目、验收条件、验证运行、记忆、决策、快照、Claude 事件），并明确这些变更不在备份中、恢复后只保留在回退副本。

### 5.1 启动恢复

应用启动、打开数据库前读取 `restore-journal.json`：

| journal.stage | 行为 |
|---|---|
| `prepared` | 未开始切换，原库保持不动；归档为 abandoned |
| `switching` / `switched` / 未完成 | 自动回退到原数据库并验证；成功归档为 recovered |
| `completed` | 校验当前数据库；通过则归档完成；失败则走回退 |
| 回退失败 | `needs_manual`；不删除任何原库或回退文件 |

## 6. 失败矩阵

| 阶段 | 失败表现 | 用户可见状态 | 数据状态 |
|---|---|---|---|
| 备份目标不可用/无权限 | `IO_ERROR` | 备份未完成 | 当前库不变，无登记 |
| 目标空间不足 | `BACKUP_ERROR` | 备份未完成 | 当前库不变，partial 清理 |
| 备份被取消 | `BACKUP_CANCELLED` | 备份已取消 | 不留下可误认的有效备份 |
| 快照完整性检查失败 | `BACKUP_ERROR` | 备份未完成 | 不登记，删除 partial |
| 备份期间数据库被长期独占 | `BACKUP_ERROR` | 备份未完成 | 30 秒超时，partial 清理 |
| 清单/SHA/路径/版本无效 | `BACKUP_ERROR`/`BACKUP_INCOMPATIBLE`/`UNSAFE_PATH` | 预检失败 | 当前库不变 |
| 隔离恢复失败 | 预检 `ready=false` | 阻止正式恢复 | 当前库不变 |
| 预检过期/令牌失效 | `RESTORE_ERROR` | 需重新预检 | 当前库不变 |
| 存在运行中的验证 | `CONFLICT` | 不能恢复 | 当前库不变 |
| 正式恢复容量不足 | `RESTORE_ERROR` | 不能恢复 | 当前库不变 |
| 创建回退副本失败 | `RESTORE_ERROR` | 恢复未开始 | 当前库不变 |
| 切换前写 journal 失败 | `RESTORE_ERROR` | 恢复未开始 | 当前库不变，原库重开 |
| 切换复制/校验失败 | `rolled_back` 或 `rollback_failed` | 明确显示回退状态 | 优先保留原库；回退失败保留全部文件 |
| 切换后最终校验失败 | `rolled_back` 或 `rollback_failed` | 明确显示未完成 | 自动回退；失败时人工检查 |
| 进程崩溃 | 启动恢复通知 | 下次启动显示 abandoned/rolled_back/needs_manual | 由 journal 与回退目录决定 |

## 7. 容量与资源上限

- 单个备份数据文件 ≤ 2 GiB；清单 ≤ 256 KiB。
- 备份前检查目标磁盘可用空间 ≥ 当前 DB 主文件 + WAL + 8 MiB；数据保护页显示策略目标当前可用空间，后端在创建时再次检查。
- 预检复制前检查应用数据目录 ≥ 数据文件 + 16 MiB。
- 正式恢复前检查峰值空间 ≥ 2 × 当前 DB + 2 × 数据文件 + 16 MiB（隔离、回退、切换与当前库）。
- 隔离副本、恢复工作区与验证工作区都位于应用数据目录，大小有界，启动时清理应用自有临时目录。
- 测试与 E2E 使用独立临时目录；不把大型备份默认写到 C 盘；备份目标由用户控制。

## 8. 隐私与导出边界

- 备份不加密；UI、Manifest 说明与数据资产清单明确写出。
- 备份不包含 Hook 原始 JSON、Prompt、transcript、工具完整输入输出、命令原文、文件内容、API Key、Salt。
- 恢复预检只显示项目名称、计数、缺失路径数量与备份之后的变更计数；不显示记忆/证据正文。
- 错误消息与日志只包含稳定错误码与固定文案；不记录绝对路径、SQL、密钥或原文。
- 数据保护页显示本机应用数据目录与备份目录，供用户定位；不上传、不自动复制到剪贴板。
- 没有“清空全部数据”按钮；备份删除、自动清理与未来数据删除必须分别授权与预览。

## 9. 未实现与已知限制

- 备份加密与密钥管理：未实现；需要新 ADR 与成熟加密库。
- 自动备份的常驻调度：未实现；只在应用启动/打开数据保护页时检查。
- 跨机器路径迁移：恢复后可能显示缺失项目路径；不自动重写路径。
- 备份来源认证/签名：未实现；SHA-256 不防伪造来源。
- 恢复后的 Hooks 自动复核：只显示提示，不自动安装/停用/卸载。
- 屏幕阅读器、真实用户剪贴板、生产安装包 sidecar、macOS/Linux 未验证。
- 人工视觉仍为 `VISUAL_REVIEW_PENDING`；未自动截图。
- SQLite 主库与伴随文件仍使用路径式 VFS。应用数据目录的祖先归属、权限、ACL 与卷检查覆盖打开时点；同 UID 进程、root/管理员进程在连接存续期替换目录项的残余风险未关闭。完整结论见 `target/v1.0.0-security-closure-20261126/MACOS_SECURITY_RECEIPT.md`。

## 10. API 核对依据

- SQLite Online Backup API 与 WAL：<https://www.sqlite.org/backup.html>、<https://www.sqlite.org/wal.html>
- `rusqlite 0.41.1` 的 `backup` feature 与 `Backup::{new, step, run_to_completion}`：按本机锁定源码（`Cargo.lock` 固定 0.41.1）核对；`backup` feature 已启用。
- Windows 文件属性/重解析点、`MoveFileExW`、`GetDiskFreeSpaceExW`：`windows-sys 0.61.2` 的 `Win32_Storage_FileSystem` 绑定（项目锁定版本）。
- Tauri 2.12 命令、`State`、`async_runtime::spawn_blocking`：沿用项目现有 Tauri 2.12.5 模式并在本机编译/E2E 验证；未新增插件或 capability。
- 本轮未使用联网 Context7；API 事实以锁定源码、官方 SQLite 文档与本机真实编译/测试为准。无法联网核对的未来版本行为不作为已确认事实。

## 11. 验证映射

| 要求 | 证据 |
|---|---|
| 一致性备份/WAL | `tests/backup.rs::backup_round_trip_preserves_business_rows_and_relations`、`concurrent_writes_do_not_corrupt_the_backup_snapshot` |
| 清单/校验和/路径 | `src/backup/manifest.rs` 单元测试、`tampered_payload_is_rejected...` |
| 损坏/截断/未知 Schema | `corrupted_database_with_matching_hash...`、`future_schema_backup_is_blocked...`、`older_schema_backup_is_upgraded...` |
| 取消/空间/锁定 | `cancelled_backup_leaves_no_valid_backup_directory`、容量检查、30 秒锁定超时（代码与文档；真实独占锁测试未运行） |
| 隔离恢复/切换/回退 | `backup_round_trip...`、`rollback_restores_the_original_database_after_a_failed_switch`、`startup_recovery_rolls_back_an_interrupted_switch` |
| 自动清理 | `policy_cleanup_removes_only_registered_automatic_backups` |
| Junction/路径 | `junction_backup_target_is_refused_when_environment_allows_creation`（本机真实创建 Junction 并拒绝） |
| 运行中验证阻断 | `restore_is_refused_while_a_verification_run_is_running` |
| UI 确认门禁 | `data-protection-page.test.tsx` |
| 真实 Tauri E2E | `target/phase6-audit/phase6-e2e-result.json`、`phase6-reopen-result.json`、`phase6-layout.json` |