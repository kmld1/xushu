# Claude Code Hooks 接入与数据契约

> Phase 4.1 已在 Windows 一次性项目中观测到六种真实 Hook 事件。证据与范围见 [Phase 4.1 独立验收报告](../phases/PHASE_4_1_E2E_REPORT.md)。
> 停用故障、路径校验和提交前审计见 [Phase 4.3 安全可靠性收尾](../phases/PHASE_4_3_SECURITY_CLOSEOUT.md)。
> 关联决策：`docs/architecture/ADR/0007-claude-code-hooks-integration.md`
> 核对版本：Claude Code 2.1.319；官方文档（2026-11-09）
> 视觉状态：**VISUAL_REVIEW_PENDING**（未自动截图）

## 1. 数据流

```text
用户明确授权
  │
  ▼
项目 .claude/settings.local.json
  │ Claude Code 在 6 个事件上执行
  ▼
xushu-hook.exe（同步 command Hook，command + args 的 exec 形式）
  │ 1. 有界读取 stdin（≤ 256 KiB）
  │ 2. 白名单字段过滤；不读取/不保存正文
  │ 3. 带本地盐 SHA-256 伪名化
  │ 4. 校验 cwd 位于授权项目根目录
  │ 5. 发布 pending/<project_id>/<timestamp>-<delivery>.json
  ▼
<app_data>/hook-inbox/
  │
  ▼
桌面应用启动 / 项目详情刷新 / 打开活动 Sheet
  │ ClaudeIntegrationService::import_pending
  │ 校验信封、项目 ID、项目根指纹、采集代次、投递幂等、语义去重
  ▼
SQLite 迁移 4–5：接入、会话、工具调用、事件、采集代次和根目录身份
  │
  ▼
项目详情“Claude Code 接入”与活动 Sheet
```

**始终不发生变化：** 任务状态、验收条件、证据、完成门禁、Git 仓库。

## 2. Hook 安装内容

每个项目注册六个事件。事件名与 Claude Code 官方名称一致：

| 事件 | matcher | 说明 |
|---|---|---|
| `SessionStart` | `*` | 会话开始、恢复、清空、压缩、分叉 |
| `PreToolUse` | `*` | 工具调用开始 |
| `PostToolUse` | `*` | 工具调用成功 |
| `PostToolUseFailure` | `*` | 工具调用失败或中断 |
| `Stop` | 无（该事件不支持 matcher） | 一轮回答结束 |
| `SessionEnd` | `*` | 会话结束 |

处理器形状：

```json
{
  "type": "command",
  "command": "<absolute path to xushu-hook.exe>",
  "args": [
    "--xushu-managed",
    "<project uuid>",
    "--project-root",
    "<canonical project root>",
    "--inbox",
    "<app data>/hook-inbox",
    "--capture-generation",
    "<positive integer>"
  ],
  "timeout": 5
}
```

- 使用 `command` + `args` 的 exec 形式，不经过 shell；路径可包含空格、中文与 `&`。
- `--xushu-managed` + 项目 UUID 是卸载时的唯一识别标记。
- 每次新启用绑定新的采集代次。已启用的旧版配置及不带代次的信封按代次 0 兼容，重新启用后使用新代次。
- `timeout = 5` 秒是上限；SessionEnd 的默认总预算为 1.5 秒，单个 Hook 的 timeout 会提高预算。CLI 正常在毫秒级退出。
- 默认**不启用**；只有用户阅读预览、勾选确认并点击“安装并启用”后才写入。

## 3. 最小事件信封

spool 中的信封示例（值为示意）：

```json
{
  "version": 1,
  "deliveryId": "0b6e6e65-5b6c-4eb0-9f0d-0b1f7e1e8f11",
  "projectId": "c3078cae-1a6f-4506-ab7d-09f5b9307710",
  "captureGeneration": 1234,
  "projectFingerprint": "<64 hex>",
  "capturedAt": "2026-11-09T00:00:00.000Z",
  "eventType": "post_tool_use",
  "sessionHash": "<64 hex>",
  "promptHash": "<64 hex or null>",
  "toolUseHash": "<64 hex or null>",
  "toolName": "Bash",
  "toolCategory": "shell",
  "failureKind": null,
  "isInterrupt": null,
  "durationMs": 42,
  "source": null,
  "endReason": null
}
```

字段规则：

- `version` 固定 1；未知字段被 `deny_unknown_fields` 拒绝。
- `deliveryId` 每次 CLI 调用生成 UUID；导入端唯一约束保证同一 spool 文件只导入一次。
- `sessionHash` / `toolUseHash` / `promptHash` 使用不同 domain 字节的带盐 SHA-256；spool 与数据库均不出现原始 ID。
- `projectFingerprint` 是 canonical 项目根路径的带盐哈希；导入时重新计算，防止同名不同位置项目混淆。
- `captureGeneration` 来自受管命令参数。导入器核对设置和数据库中的当前代次，防止撤销后的旧投递在重新启用时进入活动。
- `capturedAt` 是 CLI 接收时间（官方 Hook 输入不带事件时间戳），UI 标注为“接收时间”。
- `toolName` 仅接受 64 字符内的 ASCII 字母、数字、`_`、`-`、`.`、`:`。
- `failureKind` 枚举：`exit_code_nonzero`、`command_not_started`、`tool_error`、`timeout`、`interrupted`、`unknown`。
- `source` 枚举：`startup`、`resume`、`clear`、`compact`、`fork`、`unknown`。
- `endReason` 枚举：`clear`、`resume`、`logout`、`prompt_input_exit`、`other`、`unknown`。

**绝不写入 spool 的字段：** `transcript_path`、`cwd`、`tool_input`、`tool_response`、`error` 文本、`last_assistant_message`、`prompt_id` 原文、`session_id` 原文、`tool_use_id` 原文、模型名、API Key/Token/环境变量。

## 4. 官方输入字段使用表

| 事件 | 读取 | 明确忽略 |
|---|---|---|
| `SessionStart` | `session_id`、`cwd`、`source` | `model`、`transcript_path`、`session_title`、缓存/成本字段 |
| `PreToolUse` | `session_id`、`cwd`、`prompt_id`、`tool_name`、`tool_use_id` | `tool_input`、`permission_mode`、`transcript_path` |
| `PostToolUse` | 同上 + `duration_ms` | `tool_response`、`tool_input`、`transcript_path` |
| `PostToolUseFailure` | 同上 + `is_interrupt`、`error`（仅首行分类，不保存） | 错误正文、`tool_input`、`transcript_path` |
| `Stop` | `session_id`、`cwd`、`prompt_id` | `last_assistant_message`、`stop_hook_active`、`background_tasks` |
| `SessionEnd` | `session_id`、`cwd`、`reason` | 其他字段 |

官方字段可能随版本演化；本表对应 Claude Code 2.1.319 的参考文档与本地实测环境。

## 5. 状态与去重

- **会话**：同一项目 + `session_hash` 唯一。缺失 `SessionStart` 时 `startObserved = false`；缺失 `SessionEnd` 时 UI 显示“结束未确认”，不推断成功。
- **工具调用**：同一会话 + `tool_use_hash` 唯一。
  - 只有 `PreToolUse` → 工具调用为 `running`，时间线开始行显示“已观察”；
  - `PostToolUse` → `succeeded`；
  - `PostToolUseFailure` → `failed`；
  - 相互矛盾的成功/失败事件 → `unknown` + `conflict = true`；
  - `SessionEnd` 时仍在运行 → `unknown`，`finishObserved = false`。
- **事件幂等**：
  - `delivery_id` 唯一：同一个 spool 文件重复导入不重复记账；
  - 工具事件语义键 `<project>|tool|<sessionHash>|<eventType>|<toolUseHash>`：同一完成事件重复投递只保留一条；
  - `Stop` 有 `prompt_id` 时按 prompt 哈希去重；
  - `SessionStart` 允许恢复/分叉产生多次记录。

## 6. 目录与保留

```text
<app_data>/
├── xushu.db                         # SQLite（WAL）
├── hook-inbox/
│   ├── .xushu-salt                  # 32 字节随机盐；不通过 IPC 暴露
│   ├── pending/<project_id>/*.json  # 待导入信封
│   └── tmp/                         # 原子写入临时文件
└── hook-backups/<project_id>/      # 设置备份最多 5 份，另有设置操作锁
```

- spool 每文件上限 16 KiB，原始 stdin 上限 256 KiB，每项目待导入上限 1000 条。接收进程按项目串行检查上限并发布文件。
- 常规发布先写临时文件，再重命名。Windows 拒绝同卷重命名并返回错误 17 时，接收器使用独占句柄创建最终文件，导入器遇到共享冲突时等待下次刷新。进程异常中断留下的不完整内容由信封校验拒绝。
- 事件保留 30 天，每项目最多 5000 条事件、500 个会话。对正常可读的接入项目，即使没有新事件，启动或刷新也会检查保留上限。已停用的接入同样适用。
- 停用和卸载先移除并回读项目设置，随后写入数据库状态。若数据库写入失败，设置保持关闭，返回部分成功提示。数据库恢复后可重试原操作同步记录。
- 撤销时不消费待处理事件。已入库历史保留。后续刷新会忽略并清理已停用或旧代次的可读信封，仍被接收器占用的文件等待后续刷新。

## 7. 安全边界与失败行为

- 安装/卸载只接受 `approved = true`；前端不能提交任意命令、路径或文件内容。
- 授权路径与解析路径按路径组件比较。预览绑定根目录、设置目录、文件身份及原始字节。写入、备份和补偿前重新核对路径，拒绝链接、重解析点与身份漂移。Windows 在操作期间持有已校验目录的句柄。
- 安装和修复在备份后原子替换并回读。记录写入失败时，仅在设置仍为本次写入的文件且内容一致时补偿。停用和卸载失败后可重试，避免补偿操作重新启用采集。
- Hook CLI 对所有错误安静退出 0，不写 stdout/stderr；不会阻塞 Claude Code。
- 未授权项目、项目根指纹不匹配、无效/超大信封、超过容量的 spool 文件会被拒绝或丢弃，不会进入事件表。
- 设置备份可能包含用户自己的 `env` 值；只存放在应用数据目录，不导出、不记录日志、不通过 UI 展示内容。
- 观察失败、事件缺失、会话结束未确认均显示为未知/不完整，绝不显示为成功。

## 8. 在已获授权的一次性项目复验

Phase 4.1 的执行结果见独立报告。以下步骤仅用于明确授权的一次性测试项目。

1. 创建一个一次性 Git 测试仓库（例如 `target/phase4-manual/`），导入 XUSHU。
2. 打开项目详情 → “Claude Code 接入” → “查看并安装”：
   - 确认目标文件是 `.claude/settings.local.json`，不是团队共享文件；
   - 确认六个 Hook、命令路径、数据边界；
   - 勾选确认后点击“安装并启用”。
3. 检查测试项目 `.claude/settings.local.json`：
   - 只新增 XUSHU 条目；
   - 其他 Hooks、权限、env 未被改动；
   - 应用数据目录出现备份。
4. 在该测试项目目录启动 `claude`，执行一次只读工具调用（例如让 Claude 读取一个测试文件），然后结束一轮回答并退出。
5. 回到 XUSHU 项目详情，点击“刷新”或“查看活动”：
   - 应看到会话开始、工具开始/成功或失败、一轮回答结束、会话结束；
   - 若缺少结束事件，应显示“结束未确认”，不能显示成功。
6. 验证隐私：
   - 在 spool 与 SQLite 中搜索测试用的 Prompt、命令、文件内容、`transcript_path` 和测试 API Key，应无结果；
   - 事件只包含白名单元数据。
7. 回到项目详情，点击“停用事件采集”，再次运行 `claude`；XUSHU 不应收到新事件。
8. 点击“卸载接入”，比较 `.claude/settings.local.json` 与安装前备份：其他配置应保持原样，XUSHU 条目应消失。
9. 记录实际 Claude Code 版本、命令输出、测试结果与任何异常。

## 9. 已知限制

- **生产 sidecar（Phase 7A 起）**：正式构建通过 Tauri `externalBin` 把 `xushu-hook.exe` 安装到主程序同目录；开发构建仍要求它在 `target/debug`/`target/release` 同目录，或通过 `XUSHU_HOOK_BIN` 指向。主程序启动时执行固定的 `xushu-hook --version` 探测；缺失、损坏或版本不一致时 Claude 接入显示“接收程序不可用”。升级后 sidecar 路径/摘要变化会显示需要重新安装 Hooks。卸载应用不会移除项目中的既有 Hooks 配置，需先在应用内逐项目卸载，或人工清理带 `--xushu-managed` 的条目。
- **兼容范围**：已实测 Windows 11 与 Claude Code 2.1.319。其他版本与平台尚待验证。
- **项目归档不自动停用**：归档项目仍保留已安装 Hooks，需用户手动停用/卸载。
- **会话结束缺失**：Hook 是尽力观察；UI 只能显示“结束未确认”。
- **同步 Hook 开销**：本机 release 构建 30 次平均约 14.5 ms/次；防病毒/慢磁盘可能更高。
- **平台**：仅在 Windows 11 x64 + Claude Code 2.1.319 核对；macOS/Linux 与旧版本未验证。
- **视觉**：未自动截图；人工视觉复核仍为 `VISUAL_REVIEW_PENDING`。

## 10. Phase 6 备份/恢复边界

- 数据库备份只包含已导入的白名单 Hook 元数据；不包含 Hook 原始 JSON、Prompt、transcript、工具完整输入/输出、命令原文、Salt 或 Spool。
- Hook Spool、Salt 与项目 `.claude` 配置不在数据库备份内，恢复不会重建、清理或替换它们。
- 恢复数据库不会自动安装、停用、启用或卸载任何项目 Hooks；恢复预检与结果提示需要人工重新检查接入状态。
- 恢复后如果 Spool 仍有待导入事件，下次刷新/启动导入会写入恢复后的数据库；这不是“恢复时实时导入”。
- Hook 事件仍不是验收证据；恢复后的任务状态/完成门禁按备份时点数据库内容与后续独立验证决定。
- 详细数据资产与威胁模型见 `docs/architecture/DATA_PROTECTION.md`，决策见 ADR-0009。

## 11. 参考

- Hooks reference：<https://code.claude.com/docs/en/hooks>
- Hooks guide：<https://code.claude.com/docs/en/hooks-guide>
- Settings：<https://code.claude.com/docs/en/settings>
- Settings reference：<https://code.claude.com/docs/en/settings-reference>
- ADR-0007：`docs/architecture/ADR/0007-claude-code-hooks-integration.md`
