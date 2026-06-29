# My Agent Assets 最终可用版本 Goal 文档

## 0. 给 Codex 的总指令

Read `AGENTS.md` first.

你的目标是把 My Agent Assets 实现成一个**最终可人工完整验收的本地优先多 Agent 资产管理客户端**。

这不是开放式探索任务。你必须按本文档的目标、边界、阶段和验收标准长期执行。
Gate 是需求顺序和验收检查点，不是必须逐 Gate 停顿、提交或推送的执行单元。
整体任务在同一长期分支由主智能体连续实施，有依赖的工作按顺序推进，不使用子智能体。
不要在 Gate 之间等待用户继续指令，除非遇到无法自行解决的环境阻塞或需求矛盾。

长期实施使用分支：

```text
codex/final-product-v1
```

- 从最新 `main` 创建该分支。
- 实施过程中持续验证；完成一组连贯能力后可以创建少量有意义的提交并推送该分支。
- 不 force push，不自动合并 `main`。
- macOS Beta 验收通过后再创建 release PR 或按用户指令合并。

---

## 1. 最终产品目标

最终产品是一套本地优先的 Agent Assets Manager。

核心目标：

```text
资产中心只有一份资产。
用户可以把同一份资产按兼容规则挂载到任意 Claude Code / Codex / 自定义目标位置。
```

不是：

```text
Claude Code 一套资产中心
Codex 一套资产中心
不同 Provider 各自管理不同资产
```

正确产品模型是：

```text
Canonical Asset Center
  ├── Skills
  ├── Commands
  └── MCP Servers

Runtime Sources
  ├── Claude Code user/project/custom paths
  ├── Codex user/project/custom paths
  └── future compatible sources

Mount Targets
  ├── Claude Code user/project/custom skill dirs
  ├── Codex user/project/custom skill dirs
  ├── Claude-compatible command dirs
  ├── Claude MCP JSON configs
  └── Codex MCP TOML configs
```

Provider 不是资产分区。
Provider 只是运行时来源和挂载目标的适配规则。

---

## 2. 必须牢记的核心原则

### 2.1 资产中心唯一源头

资产中心中每个资产只保存一份 canonical copy：

```text
~/.my-agent-assets/assets/skills/<name>/
~/.my-agent-assets/assets/commands/<name>.md
~/.my-agent-assets/assets/mcps/<name>.json
```

不要设计成：

```text
assets/claude/skills/<name>
assets/codex/skills/<name>
```

不要把 Codex 资产作为独立资产池。
不要把 Claude 资产作为独立资产池。

---

### 2.2 Runtime Source 和 Mount Target 是两回事

Runtime Source 是扫描来源，例如：

```text
~/.claude/skills
<project>/.claude/skills
~/.agents/skills
<project>/.agents/skills
~/.claude/commands
<project>/.claude/commands
~/.claude.json
<project>/.mcp.json
~/.codex/config.toml
<project>/.codex/config.toml
```

Mount Target 是挂载目标，例如：

```text
Claude Code user skills
Claude Code project skills
Codex user skills
Codex project skills
Claude-compatible command directory
Claude MCP JSON config
Codex MCP TOML config
Custom skill directory
```

一个资产可以被挂载到多个兼容目标。

---

### 2.3 兼容矩阵

必须遵守：

| Asset Type | Claude Code Target | Codex Target | Custom Target |
|---|---:|---:|---:|
| Skill | 支持 | 支持 | 支持 skill directory |
| Command | 支持 | 不支持 | 仅支持 Claude-compatible command dir |
| MCP Server | 支持 JSON adapter | 支持 TOML adapter | 仅在明确 adapter 时支持 |

Codex 不支持 Command 资产，因此不要实现 Codex Command mount。
不要自动把 Command 转成 Codex Skill，除非未来用户明确要求 migration 功能。
本目标内不做 Command -> Skill 转换。

---

### 2.4 MCP 统一模型与 Live Config 编译

MCP 使用文件型统一模型，不引用 SQLite，也不建立数据库、DAO 或数据库缓存层。

MCP server 的 canonical definition 是可通过 Git 同步的唯一真实配置：

```text
~/.my-agent-assets/assets/mcps/<name>.json
```

该 JSON 文件保存 canonical MCP model，不是 Claude Code 或 Codex live config 的原样副本。统一模型使用：

```json
{
  "schemaVersion": 1,
  "name": "postgresql",
  "spec": {
    "type": "stdio",
    "command": "npx",
    "args": ["-y", "@example/postgresql"],
    "env": {}
  },
  "providerExtensions": {}
}
```

该模型参考 cc-switch 的 `McpServer.server: serde_json::Value`，但字段校验和输出格式以当前 Claude Code / Codex 官方实现为准：

- `spec.type` 使用 Claude/社区 JSON 命名，不使用自定义 `transport` 字段。
- `type` 省略时按 `stdio` 处理。
- `stdio`：要求非空 `command`，支持字符串数组 `args`、字符串 map `env`，可保留 `cwd`。
- `http`：要求 `url`，支持字符串 map `headers`。
- `sse`：作为 Claude-compatible transport 保留；当前 Codex adapter 不支持时必须 blocked，不能伪装成 HTTP 或静默丢弃。
- `providerExtensions` 保存无法安全映射为通用字段的 provider-specific 配置，并按 provider 隔离。
- 未知字段可以保留用于 round-trip，但任何不能无损编译到目标的字段都必须在 Preview 中列出；影响行为的字段不能静默丢失。

Claude Code 和 Codex 的 live config 只是 canonical definition 的编译产物，不是资产源头：

```text
canonical MCP
→ Claude Code ~/.claude.json mcpServers.<name>
→ Codex ~/.codex/config.toml [mcp_servers.<name>]
```

不得软链整个：

```text
~/.claude.json
~/.codex/config.toml
```

target 的启用和挂载状态是机器本地状态，记录在本地 target registry / `mounts.yaml` 中，不写入可同步的 MCP asset JSON。
状态使用 `targetId + enabled` 表达，不使用只能表示固定客户端的 `enabled_claude / enabled_codex` 数据库字段。

MCP 实现必须分层：

```text
file repository
→ MCP service
→ Claude JSON renderer / Codex TOML renderer
```

- file repository 负责 canonical asset 和机器本地 target binding。
- MCP service 负责 import、upsert、toggle、sync、remove、delete 的业务编排。
- renderer 只负责目标格式的读取、精确 patch、删除和原子写入。
- service 不直接处理 JSON/TOML 细节。

Claude renderer：

- user scope 只 patch `~/.claude.json` 根对象的 `mcpServers`。
- local scope 只 patch `~/.claude.json.projects["<canonical project path>"].mcpServers`。
- project scope 只 patch `<project>/.mcp.json` 根对象的 `mcpServers`。
- 启用时 upsert 当前 server，禁用时只删除当前 server。
- 保留所有其他字段、project entries 和未选中的 MCP server。
- 写入前移除 `enabled/source/id/name/description/tags/homepage/docs` 等内部字段。
- 对 `stdio/http/sse` 输出 Claude Code 支持的 `type/command/args/env/url/headers`。
- provider-specific Codex extensions 不得写入 Claude live config。
- Windows 下将 `npx/npm/yarn/pnpm/node/bun/deno` 等命令按需要编译为 `cmd /c`。

Codex renderer：

- 使用 `toml_edit` 只 patch `[mcp_servers]`。
- 保留其他配置、未选中的 server 和原有注释。
- stdio 输出 `command/args/env/cwd`；streamable HTTP 输出 `url`。
- 将 canonical `headers` 编译为 Codex `http_headers`。
- 支持 Codex 官方字段时可输出 `env_vars/env_http_headers/bearer_token_env_var/startup_timeout_sec/tool_timeout_sec/enabled_tools/disabled_tools`。
- 不向 Codex 输出 `type`；transport 由 `command` 或 `url` 决定。
- canonical `sse` 对当前 Codex target 返回 incompatible/blocked。
- 禁用时删除 `[mcp_servers.<name>]`。
- 删除时同时清理旧错误格式 `[mcp.servers.<name>]`。

不得照搬 cc-switch 对任意未知 JSON/TOML 字段的宽松浅层转换。目标 adapter 必须使用明确字段 allowlist，并以当前客户端官方 schema 和回归 fixture 为准。

Import 已有 Claude/Codex MCP 配置时：

- 只读取 live config 并写入 canonical model。
- 记录来源 target 的本地启用状态。
- 不自动反向写入任何 live config。
- 只有用户显式执行 upsert、toggle、mount 或 sync 时才编译 live config。

禁用某个 target 时，只从该 target 的 live config 精准删除当前 server。
删除 MCP asset 时，先从所有曾启用的 target live config 中删除；所有目标处理完成后再删除 canonical asset。

本版本不区分 MCP spec 中的普通值与敏感值，Import 按原配置写入 canonical definition。
由于 canonical MCP 可能包含密码、token 或 Authorization header，Git Push 必须受“仅 GitHub Private 仓库”强校验约束，日志和错误输出仍不得打印配置值。

---

### 2.5 Windows 挂载机制

Windows 不得把权限失败静默降级为文件复制，也不使用 hardlink 代替 canonical mount。

- Skill directory 使用 directory junction。
- Command file 使用 file symlink。
- MCP 使用 JSON/TOML renderer patch，不使用链接。
- Command file symlink 权限不足时阻止 apply，并提示开启 Windows Developer Mode 或使用具备相应权限的环境。
- 不使用 hardlink：hardlink 不能跨卷，并可能在 canonical 文件原子替换后与资产中心内容分叉。
- Preview 必须显示将采用的 mount mechanism、权限检查结果和阻断原因。
- `doctor` 必须检测 junction、file symlink、目标文件系统和相关权限。

macOS/Linux 继续使用 directory/file symlink。所有平台都必须拒绝 path traversal、symlink/junction escape 和未经批准的目标路径。

---

### 2.6 Preview、并发与原子写入

所有真实写操作必须绑定有效 preview，不得把 preview 仅作为 UI 展示。

- Preview 记录规范化输入摘要及所有受影响文件的 SHA-256 fingerprint。
- SHA-256 仅用于 stale-preview 和并发变化检测，不用于自动合并、自动去重或自动解决冲突。
- GUI `previewId` 默认有效期为 10 分钟，只存在于当前后端进程。
- Apply 必须重新读取并校验 fingerprint；任一路径发生变化时返回 `stalePreview`，禁止写入并要求重新预览。
- CLI preview/apply 在同一个命令进程内完成，不依赖跨进程 preview cache。
- 写操作按规范化路径排序加锁，避免 GUI、CLI 和多个窗口并发修改，也避免锁顺序死锁。
- 获得锁后再次校验 fingerprint，再创建 backup，再执行写入。
- JSON、TOML、registry 和其他结构化文件使用同目录临时文件、flush/fsync 和 atomic rename。
- 多目标 MCP 删除或同步使用 operation journal；中途失败时回滚已完成目标。
- 无法完整回滚时保留 journal，返回明确的 partial failure 和人工恢复信息。
- 不得把部分成功作为整体成功返回。

---

### 2.7 Target Registry 与路径授权

Target Registry 是机器本地的“已登记并允许写入的挂载目标清单”，不是数据库，也不通过 Git 同步。

```yaml
targets:
  claude-user-skills:
    kind: claude_user_skills
    path: ~/.claude/skills

  project-a-claude-skills:
    kind: claude_project_skills
    path: ~/workspace/project-a/.claude/skills
```

- Apply 只接受 `targetId`，不信任前端临时传入的真实写入路径。
- Rust 后端从本机 registry 解析最终路径，并重新验证 asset kind、target kind 和 adapter 兼容性。
- 标准 user targets 可以由应用基于 HOME 自动登记。
- project target 必须属于用户明确登记或确认过的 project root。
- custom target 首次添加时必须展示规范化绝对路径并要求显式确认。
- 写入前必须执行路径规范化、允许根目录校验和祖先 symlink/junction 检查。
- 不允许把文件系统根目录、HOME 根目录、资产中心自身或系统敏感目录直接登记为 target。
- Registry 路径变化、目标被替换或链接指向变化时，旧 Preview 立即失效。
- 删除 project/custom target 只删除登记关系，不删除目标目录或用户文件。
- 所有测试使用 fake HOME 和临时 custom root。

该机制的目的，是防止前端 Bug、篡改请求或错误 DTO 让 Rust 后端写入任意文件路径。

---

### 2.8 GUI 与 CLI 共用 Rust Core

`crates/core` 是 Scan、Import、Adopt、Mount、Conflict、Backup、Target Registry 和 MCP renderer 的唯一业务实现。

- Tauri commands 只负责 DTO 转换、桌面进程状态和调用 core。
- CLI 只负责参数解析、终端交互、输出格式和调用 core。
- GUI `previewId` 的进程内缓存可以位于 Tauri 层，但 preview 生成、fingerprint 和 apply 校验必须属于 core。
- `app_info`、窗口行为等桌面专属能力继续留在 Tauri。
- GUI 与 CLI 可以使用不同展示 DTO，但业务结果必须来自同一 core。
- 不允许在 `apps/desktop/src-tauri` 和 CLI 中分别实现两套文件操作或 provider adapter。
- 共享 core 的边界整理必须在 Codex 写入、统一 MCP renderer 和新 target apply 之前完成，不能等最终阶段再对齐。

---

### 2.9 固定资产中心路径

V1/V2 的资产中心根目录固定为：

```text
~/.my-agent-assets
```

- Settings 只读展示该路径，不允许编辑。
- 不实现资产中心迁移、选择其他磁盘或自定义 root。
- GUI、CLI 和 Rust core 使用同一固定解析规则。
- 测试只能通过显式 fake HOME 改变解析结果，不能接触真实 HOME。
- 不得保存一个看似可配置但后端不生效的 `assetCenterPath`。

---

### 2.10 Git 同步白名单与双层备份

Git 同步使用明确白名单，不执行无约束的 `git add .`。

允许同步：

```text
assets/
assets.yaml
backups/portable/
必要的 schema/version 文件
```

必须保持机器本地并写入 `.gitignore`：

```text
backups/local/
mounts.yaml
targets.yaml
config.local.yaml
operations/
locks/
cache/
logs/
secrets/
```

备份分为：

- portable backup：用户显式创建的 canonical assets 快照，包含可移植 manifest、相对路径、schema version 和可选 Git commit 信息，可以通过 Git 跨设备同步。
- local backup：Import、Mount、Adopt、MCP patch 等 apply 前自动创建的 runtime 安全备份，用于用户按教程手动恢复。

portable backup 不得包含本机绝对路径、target binding 或 runtime live config；它可以包含 canonical MCP 原文，因此与 canonical assets 使用相同的 Private remote Push 限制。
local backup 可以保存手动恢复所需的原 runtime 文件，但不得进入 Git。

App 不提供自动 Restore：

- 不提供 `preview_restore`、`restore_apply` 或其他恢复写命令。
- GUI 只展示 backup history、manifest、创建原因、时间、文件位置和受影响路径。
- GUI 可以提供“在文件管理器中显示”和只读的手动 Restore 教程。
- CLI 不提供 `maa restore`。
- 手动教程必须要求先退出相关客户端、再次复制当前文件作为保护，并只操作 manifest 明确列出的路径。
- operation journal 自身的自动 rollback 属于单次失败事务处理，不是面向用户的 Restore 功能。

备份保留策略：

- Portable backup 永不自动删除。
- Local backup 默认也不自动删除。
- Backup History 显示数量、总大小和最旧时间。
- 用户可以手动删除选中的 backup，删除前高亮说明影响。
- 正被 operation journal 引用的 backup 禁止删除。
- 超过可配置容量提醒阈值时提示清理，默认提醒阈值为 `1 GB`。
- V1 不按天数自动清理。
- App 卸载不删除任何 backup。

从另一设备 pull 后只获得 canonical assets 和 portable backups，不自动创建本机 mount。
远端删除 canonical asset 后，本机已有 binding 标记为 orphaned，必须由用户选择卸载、重新绑定或保留；不得静默删除或忽略。

---

### 2.11 GitHub Private Remote 强校验

本版本允许 canonical MCP 和 portable backup 原样进入 Git，但远程 Push 只支持能够通过认证 API 验证为 Private 的 GitHub repository。

- 本地使用、commit 和无 remote 场景不要求 GitHub。
- `sync push` 前必须读取当前 remote，并通过用户本机已有的 `gh` 登录状态或 GitHub API 凭证查询 repository visibility。
- 只有明确返回 `PRIVATE` 才允许 Push。
- Public、Internal、认证失败、API 不可用、remote 无法识别或隐私状态未知时，一律 fail closed，禁止 Push。
- 每次 Push 都重新验证，不能永久缓存结果。
- Preview 后 remote URL、repository identity 或 visibility 变化时，旧 Preview 失效。
- V1 不支持无法验证隐私状态的 GitLab、自建 Git 或普通 SSH remote Push。
- App 不提供 GitHub 登录、OAuth、账号绑定或 token 管理，只使用本机已有 Git/`gh` 环境。
- UI 必须明确提示：Private repository 仍会向所有仓库成员暴露配置，Git history 也可能长期保留已提交内容。
- 日志、错误、诊断报告和 operation journal 不得输出 MCP 配置值或认证凭据。

---

### 2.11.1 Fail-safe Git Sync

Git Sync 只由用户显式触发，Scan、Import、Mount、Adopt 和普通启动不得自动 Pull/Push。

Pull：

- 先 fetch 并生成同步 Preview。
- 工作区不干净时阻止，不自动 stash。
- 只允许 fast-forward 更新。
- 本地与远端分叉时阻止，并展示提交差异和手动处理教程。
- 不自动 merge、rebase、reset 或解决 Git conflict。

Push：

- 只 stage Git 同步白名单。
- Preview 展示待提交文件和 commit message。
- 用户确认后 commit。
- Push 前再次验证 GitHub repository visibility 为 `PRIVATE`。
- 只允许普通 Push，禁止 force push。
- 远端领先或分叉时阻止，要求先完成安全 Pull/手动处理。

通用规则：

- Git 操作使用仓库锁和 stale-preview 校验。
- 不自动修改用户已有 remote 或 branch。
- Git conflict 不进入资产同名 Conflict Resolver，本版本只显示状态和手动处理教程。
- App 不执行 `stash/reset/rebase/force push`。

---

### 2.12 Provider 初始化与目标创建

Scan/Import 永远不创建客户端目录或配置。

- Claude user target 只有在 `~/.claude/` 或 `~/.claude.json` 已存在时视为已初始化。
- Codex user target 只有在 `~/.codex/` 或 `~/.codex/config.toml` 已存在时视为已初始化。
- 只检测到可执行文件但配置目录不存在时，状态为“已安装但未初始化”，不得自动写入。
- 未初始化的 user target 在 Preview 阶段返回 blocked，并提示先启动对应客户端完成初始化。
- 已初始化但具体 MCP 配置文件不存在时，可以在明确 Preview 后创建文件；Preview 必须列出创建动作。
- project/custom target 可以创建缺失的 skill/command 目录或 MCP 配置，但必须先经过 Target Registry 授权，并在 Preview 中明确展示将创建的目录和文件。
- 删除或禁用最后一个 MCP server 时，只删除对应 entry，不删除客户端配置文件或目录。
- Import 已有配置只读，不会因发现 provider 而创建其他 provider 配置。

---

### 2.13 文件型状态模型

不使用数据库。资产中心使用四个由 serde 正式解析的 YAML 文件：

```text
~/.my-agent-assets/
├── assets.yaml
├── config.yaml
├── targets.yaml
└── mounts.yaml
```

- `assets.yaml`：Git 同步的 canonical asset 索引，只保存 asset ID、type、name 和可移植 metadata，不保存来源绝对路径、target binding 或本机状态。
- `config.yaml`：机器本地 scan roots、max depth、Git branch/remote 偏好、UI、日志和 CLI 设置；固定 asset center root 不作为可编辑配置。
- `targets.yaml`：机器本地已授权 Mount Targets。
- `mounts.yaml`：机器本地 asset-to-target bindings，以及 `mounted/outOfSync/orphaned` 等状态。

统一要求：

- 每个文件必须包含 `schemaVersion`。
- 未知的新 schema version 必须停止写入并显示升级提示，不得猜测解析。
- 旧版本升级必须先创建 local backup，再执行显式 migration。
- 文件损坏时返回诊断并保留原文，不得自动重建覆盖。
- 所有写入遵守 ordered lock、stale revalidation 和 atomic replace。
- `config.yaml`、`targets.yaml`、`mounts.yaml` 必须加入 `.gitignore`。
- 禁止使用自定义管道分隔文本伪装成 YAML。
- 不引入 SQLite 或其他数据库。

---

### 2.14 统一项目扫描深度

GUI 与 CLI 必须调用共享 core 的同一项目发现逻辑：

- 使用 `config.yaml` 中的 `scan_roots`。
- 使用可配置的 `max_depth`，默认值为 `5`。
- Desktop 当前只扫描根目录下一层的实现必须收敛到共享 core，不保留与 CLI 不同的发现语义。
- 继续沿用项目已经固定的目录跳过、不跟随目录 symlink 和读取失败降级规则，不在本目标中重新设计扫描算法。

---

### 2.15 显式初始化与首次启动零写入

安装 App 和首次启动只检测环境，不创建资产中心，也不修改 Claude、Codex 或 Git 配置。

资产中心未初始化时，Dashboard 显示明确空状态。用户显式执行 GUI“初始化资产中心”或 `maa init` 后，调用共享 Rust core 的同一初始化逻辑。

初始化 Preview/Apply 创建：

```text
~/.my-agent-assets/
├── assets/skills/
├── assets/commands/
├── assets/mcps/
├── backups/portable/
├── backups/local/
├── assets.yaml
├── config.yaml
├── targets.yaml
├── mounts.yaml
└── .gitignore
```

- 初始化本地 Git repository，默认分支为 `main`。
- 不自动添加 remote、commit 或 push。
- 初始化必须幂等；已有合法文件不覆盖。
- 目录存在但结构损坏或 schema 不兼容时停止并显示诊断。
- 初始化不扫描、不导入、不挂载资产。
- 卸载 App 不删除 `~/.my-agent-assets`。
- 测试只允许在 fake HOME 执行初始化。

---

### 2.16 来源解耦与本机导入历史

Canonical asset 不保存来源身份。

- `assets.yaml` 不保存原始绝对路径、source provider、原项目名称或原 runtime target。
- Asset identity 始终只有 `asset kind + name`。
- 来源不参与命名、兼容矩阵、挂载或 Git 同步。

机器本地 operation history 可以记录导入时间、source provider、source path 和 source scope，用于诊断：

- 不参与资产身份或业务判断。
- 不进入 Git 或 portable backup。
- 清理历史不影响 canonical asset。
- UI 可以把“最近从哪里导入”作为可选本机历史信息，但资产详情以 canonical path 和当前本机 mounts 为准。

---

### 2.17 Asset Registry 与内容一致性

`assets.yaml` 是资产注册索引，`assets/` 下的文件/目录是实际内容，两者共同组成 canonical asset。

- Import/Delete 必须在同一 transaction 中同时更新内容和 registry。
- Registry 有记录但内容缺失时标记 `invalid`，禁止 mount/apply。
- 内容存在但 Registry 无记录时标记 `unregistered`，不自动视为正式资产。
- `doctor` 展示差异，并通过 Preview 提供修复 registry、从内容重新登记或删除孤立内容的选项。
- App 启动、Scan 和 Git Pull 后不得自动修复或覆盖。
- MCP JSON、Skill directory 或 Command file 损坏时保留原文件并报告诊断。
- Git Pull 完成后重新验证 registry/content 一致性。
- portable backup 必须同时保存对应内容和 `assets.yaml` 快照。
- 所有一致性修复遵守 backup、stale-preview、ordered lock 和 atomic write 规则。

---

### 2.18 写操作确认交互

所有写操作都必须先展示 Preview，但不要求用户手工输入 `APPLY` 或其他确认文本。

- 普通操作在 Preview 后使用明确的确认按钮。
- 高风险操作使用醒目的风险色、影响摘要、受影响路径/targets 和不可逆后果说明，再由用户点击明确确认按钮。
- 高风险操作包括 Adopt、overwrite conflict、覆盖已有 runtime 内容、卸载全部并删除资产、MCP 多目标 Sync/Delete、Git commit + Push 和 doctor consistency repair。
- 不使用隐藏确认、默认选中或自动倒计时执行。
- 后端安全不依赖 UI 确认方式；previewId、stale validation、locks、backup、journal 和 rollback 仍然强制执行。

---

### 2.19 日志、诊断与网络边界

App 不接入遥测、崩溃上报或行为分析 SDK。除用户显式执行 Git 操作和 GitHub Private visibility 校验外，不发起产品网络请求。

- 日志只保存在本机 `logs/`，并加入 `.gitignore`。
- 日志可以记录 operation type、asset ID、target ID、结果和错误类别。
- 日志不得记录 MCP JSON/TOML 原文、env/header 值、Git credentials、backup 内容或用户文件内容。
- 本机绝对路径默认缩写为 `~` 或 target ID；Debug 模式才允许显示完整路径。
- 导出诊断包前必须 Preview 文件清单。
- 诊断包默认只包含脱敏日志、App/core/CLI 版本、平台和状态摘要。
- 默认不得包含 canonical assets、live config、backup 或用户配置文件。
- 只有用户逐项明确选择后才能附带配置文件，并再次显示敏感信息风险。
- 日志按 Settings 的日志保留周期清理；backup 不受日志清理策略影响。

---

### 2.20 V1 界面语言

V1 只交付简体中文 UI，不引入 i18n 框架或语言切换设置。

- Skills、Commands、MCP、Git、Pull、Push 等行业技术名词可以保留英文。
- 错误说明、风险提示、空状态、诊断和手动 Restore 教程使用中文。
- CLI 命令与参数保持英文，说明和错误提示使用中文。
- Rust 返回稳定 error code、结构化参数和可选开发信息；前端不得通过解析中文错误字符串判断业务分支。
- 英文或其他语言版本作为后续独立里程碑。

---

### 2.21 基础无障碍验收

V1 不做完整 WCAG 或专门无障碍认证，只保留桌面端基础可用性验收：

- 关键流程可以使用键盘完成。
- 主要按钮、输入框、导航、列表项和状态具有正确可读名称。
- Tab 顺序与视觉顺序基本一致，Focus 状态清晰且不被 drag region 吞掉。
- 成功、冲突、风险和禁用状态不只依赖颜色表达。
- 系统字体放大以及 Windows 125%/150% DPI 下不遮挡关键内容和操作。
- macOS VoiceOver / Windows Narrator 能识别主要导航与操作即可，不要求全面优化或认证。

---

### 2.22 未完成事务自动回滚

operation journal 用于本应用写操作的崩溃恢复，不属于面向用户的 Restore 功能。

- Apply 开始前持久化 write-ahead operation journal。
- 每完成一个步骤立即原子更新 journal 状态。
- App/CLI 启动时检查未完成 journal。
- 存在未完成操作时允许只读查询，但阻止新的写操作。
- Rust core 自动尝试回滚到操作前状态。
- 自动回滚成功后保留 journal 和 local backup history，并显示恢复结果。
- 自动回滚失败时停止进一步修改，展示受影响路径、已完成/未完成步骤和手动恢复教程。
- 不允许把半完成事务直接标记为成功或从中间继续 Apply。
- 该机制只处理本应用未完成事务，不提供任意历史 backup restore。

---

### 2.23 Fake HOME 测试隔离

Unit、integration、E2E、Visual QA 和 Computer Use 自动化全部使用临时 Fake HOME。

- 测试通过显式环境变量或测试参数指定根目录，例如 `MY_AGENT_ASSETS_HOME=/tmp/maa-test-home`。
- 测试模式在 GUI 中持续显示“测试环境”，避免与真实数据混淆。
- 自动化禁止读取或写入真实 `~/.claude`、`~/.claude.json`、`~/.codex` 和 `~/.my-agent-assets`。
- Release 构建不默认启用测试模式。
- 真实 HOME 最终验收只由用户明确发起；自动化脚本不得替代用户确认真实写操作。
- 真实环境可以先执行只读 Scan/Import Preview，任何写操作仍需单独确认。
- 测试结束后删除临时 Fake HOME，保留脱敏测试报告和截图。

---

### 2.24 V1 资产编辑边界

- Skill 不提供内置 Markdown/目录编辑器，只读预览并支持“在文件管理器中显示”。
- Command 不提供内置编辑器，只读预览并支持使用系统外部编辑器打开。
- MCP 提供结构化新增/编辑 canonical spec，支持 `stdio/http/sse` 字段和高级 JSON 预览。
- MCP 保存只更新 canonical JSON 和 registry metadata。
- MCP 已启用 targets 在保存后标记 `outOfSync`，不得自动写 live config。
- 用户显式执行 Sync 后才通过 renderer 更新 Claude/Codex。
- MCP 编辑前后执行 schema 和 target compatibility 校验；未知或无法无损编译的字段显示 warning/blocked。
- V1 不实现通用代码编辑器、Skill 文件树编辑器或 Command Markdown 编辑器。

---

### 2.25 V1 Rename 边界

- 继续使用现有同名资产人工冲突处理，候选导入项可以选择 `skip/overwrite/rename`。
- Conflict `rename` 为候选新资产指定安全的新名称。
- 不提供已存在 canonical asset 的独立 Rename 操作。
- MCP 编辑中的 `name` / asset ID 只读。
- 用户需要改名时，创建或导入新名称、重新挂载，再按安全 Delete 流程删除旧资产。
- V1 不实现跨 canonical storage、registry、bindings 和 live configs 的事务性 Rename。

---

## 3. 正确业务语义

### 3.1 Scan

Scan 只发现运行时来源资产，不写入。

输出应表达：

```text
source type
source provider
source path / config path
asset kind
asset name
is already managed
is symlink
current target
conflict risk
import eligibility
mount/adopt eligibility
```

---

### 3.2 Import

Import 只把来源资产复制 / 抽取到 canonical asset center。

Import 不改变原生效位置。

例如：

```text
~/.agents/skills/review
→ ~/.my-agent-assets/assets/skills/review
```

Import 之后，原来的：

```text
~/.agents/skills/review
```

仍然继续生效，除非用户执行 Mount 或 Import and Adopt。

---

### 3.3 Mount

Mount 让资产中心版本在某个目标位置生效。

Skill mount：

```text
target skill directory/<name> -> asset center skill directory
```

Command mount：

```text
target command file -> asset center command file
```

MCP mount：

```text
asset center MCP canonical definition
→ Claude JSON mcpServers.<name>
```

或：

```text
asset center MCP canonical definition
→ Codex TOML [mcp_servers.<name>]
```

MCP mount 是 renderer 对 live config 的精确 patch，不是文件软链。
只有用户显式执行 mount、toggle 或 sync 时才写入 live config。

---

### 3.4 Import and Adopt

Import and Adopt 是组合动作：

```text
scan source
→ import to canonical asset center
→ backup original runtime source
→ mount asset center version back to original source location
```

也就是“导入并接管”。

对于 Skill：

```text
original runtime skill dir
→ backup
→ symlink to asset center skill dir
```

对于 Command：

```text
original runtime command file
→ backup
→ symlink to asset center command file
```

对于 MCP：

```text
original mcp entry
→ backup original config
→ replace selected server entry with managed generated entry
```

---

### 3.5 Unmount 与 Delete Asset

Unmount 只解除一个指定 target，并保留 canonical asset。

- Skill/Command 只删除仍然指向该 canonical asset 的 symlink/junction。
- 目标已被用户替换为普通文件或指向其他位置时，禁止删除并报告 conflict。
- MCP 只从指定 target live config 删除对应 server entry。

Delete Asset 删除 canonical asset：

- 删除前必须枚举全部本机 bindings。
- 存在 binding 时禁止直接删除，必须生成“卸载全部并删除”Preview。
- Preview 展示所有将解除的 targets、将修改的 live configs 和将删除的 canonical 文件。
- Apply 前创建 local backup。
- Skill/Command 安全移除全部链接后再删除 canonical。
- MCP 从所有已启用 targets 精准删除 entry 后再删除 canonical。
- 任一 target 无法处理时，整体失败并回滚，不得留下断链或部分删除。
- 删除完成后保留 backup history 和手动 Restore 教程。
- Git Pull 导致远端 canonical asset 消失时，本机 binding 标记为 `orphaned`；用户选择清理、重新绑定或保留，不自动删除 runtime 内容。

---

## 4. 严格禁止

不要实现以下内容：

- SQLite、数据库表、DAO 或数据库缓存作为 MCP 存储。
- 把 Claude/Codex live config 当作 MCP SSOT。
- 软链整个 `~/.claude.json` 或 `~/.codex/config.toml`。
- MCP import 后自动反向同步到其他 live config。
- 用固定 `enabled_claude / enabled_codex` 字段替代本地 target binding。

```text
账号
登录
OAuth 云账号管理
团队空间
订阅
Billing
云端同步
服务器同步
Prompt marketplace
Cursor rules
AGENTS.md / CLAUDE.md instruction asset 管理
Codex Command asset
Command -> Codex Skill 自动转换
大规模 UI 重设计
推翻当前 AppShell / window behavior / macOS Overlay / Windows native titlebar
```

Codex OAuth token 不由本 App 管理。
如果 Codex MCP server 需要 OAuth，只展示说明或 warning，不存储 token。

---

## 5. 长期执行方式

你必须把所有 Gate 作为一个连续实施计划执行。

执行规则：

1. 先完成模型、共享 core 和安全边界，再接 adapters、GUI、CLI、同步与打包。
2. 有前后依赖的能力按 Gate 顺序实现。
3. Rust core、frontend contracts/UI、测试/文档由主智能体按依赖顺序连续完成，不委派子智能体。
4. 每完成一组能力立即运行相关增量测试，失败则在继续前修复。
5. 不需要逐 Gate commit、push、等待用户或单独汇报。
6. 持续更新 `docs/implementation-progress.md`，记录整体完成度、验证和剩余风险。
7. 所有能力完成后运行全量验证、Fake HOME E2E、Visual QA 和安装包验收。
8. 最后整理为少量语义清晰的提交并推送 `codex/final-product-v1`。

不要因为某个 Gate 较大就跳过；可以拆成内部步骤，但最终必须覆盖全部 Gate 验收项。
如果验证失败，先自行修复，不要立即停止。
只有当环境缺失、依赖无法安装、需求冲突或连续修复后仍无法通过时，才停止并报告。

不得在实施过程中自动合并 `main` 或创建正式 release tag。

---

## 6. 每个 Gate 的通用自检

每个 Gate 结束前，检查：

```text
是否仍然是一份 canonical asset center？
是否把 provider 错误实现成资产分区？
是否错误支持了 Codex Commands？
是否误改 AppShell/window behavior？
是否引入账号/云/团队/billing？
是否保留假数据默认展示？
是否存在可点击但无效的 UI？
是否有 planOnly 写入？
是否真实 HOME 被测试污染？
是否缺少 fake HOME 覆盖？
是否未更新 docs/implementation-progress.md？
```

如果发现偏差，必须在当前 Gate 内修正后再提交。

---

## 7. Gate 0：当前状态审计与模型纠偏

### 目标

审计当前仓库，确认当前实现是否偏向“Provider 资产分区”。
如果有偏差，先改文档、类型命名和 UI 文案，把方向纠正为：

```text
一份资产中心 + 多运行时来源 + 多挂载目标
```

### 必做

- 阅读 `AGENTS.md`、`docs/*`、当前 contracts、Rust commands、frontend pages。
- 先更新 `AGENTS.md` 的产品范围：
  - 保留 Foundation Freeze、No Login、AppShell 和窗口策略。
  - 将 Codex 从“Skills/MCP read-only”更新为本文档定义的 scan/import/compatible target mount 范围。
  - 继续明确禁止 Codex Command、Codex OAuth token 管理和 AGENTS.md asset 管理。
  - 将“frontend-only/mock phase”等过期阶段性约束更新为真实 Rust core 集成规则。
- 检查是否存在误导性描述：
  - Codex provider asset center
  - Codex assets separate from Claude assets
  - Provider owns assets
- 修正文档和命名，改成：
  - runtime source
  - mount target
  - target adapter
  - canonical asset
- 创建或更新：
  - `docs/final-product-model.md`
  - `docs/implementation-progress.md`

### 不做

- 不改业务流程。
- 不新增 Codex 写入。
- 不做 UI 大改。

### 验收

- `AGENTS.md` 不再与本文档要求的 Codex compatible writes 冲突。
- 文档明确写出“一份资产中心，多目标挂载”。
- 没有文档继续暗示 Claude/Codex 各有一套资产中心。

### Commit

```text
docs(desktop): align final asset center model
```

---

## 8. Gate 1：Target Registry 与兼容矩阵

### 目标

建立统一 Mount Target 模型，让后续所有挂载都基于 target，而不是基于 provider 分支散落实现。

### 必做

新增或重构类型：

```ts
type AssetKind = "skill" | "command" | "mcp";

type RuntimeProvider = "claude_code" | "codex" | "custom";

type MountTargetKind =
  | "claude_user_skills"
  | "claude_project_skills"
  | "codex_user_skills"
  | "codex_project_skills"
  | "custom_skill_directory"
  | "claude_user_commands"
  | "claude_project_commands"
  | "custom_command_directory"
  | "claude_user_mcp_json"
  | "claude_local_mcp_json"
  | "claude_project_mcp_json"
  | "codex_user_mcp_toml"
  | "codex_project_mcp_toml"
  | "custom_claude_mcp_json"
  | "custom_codex_mcp_toml";

type MountAdapter =
  | "symlink_directory"
  | "symlink_file"
  | "windows_directory_junction"
  | "json_mcp_patch"
  | "toml_mcp_patch";

type MountTarget = {
  id: string;
  kind: MountTargetKind;
  provider: RuntimeProvider;
  accepts: AssetKind[];
  adapter: MountAdapter;
  scope: "user" | "local" | "project" | "custom";
  path: string;
  projectPath?: string;
};
```

Target Registry 使用机器本地文件保存已授权的 `MountTarget`。它不是 SQLite/数据库，不属于 Git 同步资产。
所有 preview/apply 输入引用 `targetId`；后端从 registry 解析并验证真实路径，不直接信任前端提交的 path。

Target discovery 同时返回 provider 状态：

```text
not_installed
installed_not_initialized
initialized
```

未初始化的 user target 不进入可执行 apply 状态。

MCP scope 必须明确区分：

```text
Claude user:
  ~/.claude.json -> root mcpServers

Claude local:
  ~/.claude.json -> projects["<canonical project path>"].mcpServers

Claude project:
  <project>/.mcp.json -> root mcpServers

Codex user:
  ~/.codex/config.toml -> [mcp_servers]

Codex project:
  <project>/.codex/config.toml -> [mcp_servers]
```

Claude local 和 project 是不同 target，不得合并。Codex 不虚构 Claude 式 local scope。

Custom MCP target 只允许：

- 已授权自定义 JSON 文件 + Claude-compatible 根 `mcpServers` adapter。
- 已授权自定义 TOML 文件 + Codex-compatible `[mcp_servers]` adapter。

Custom 只改变目标路径，不改变配置语义。不支持任意 JSONPath/TOML table path、YAML/XML、自定义转换脚本或未知应用格式猜测。

Rust 和 TS 合同保持一致。

### 兼容规则

- Skill 只能挂到 skill target。
- Command 只能挂到 command target。
- MCP 只能挂到 MCP adapter target。
- Codex 不存在 command target。
- 不兼容组合在 UI 中隐藏或在 preview 阶段硬拒绝。

### 测试

- TS/Rust 合同测试。
- 兼容矩阵测试。
- unsupported target 测试。

### Commit

```text
feat(desktop): define mount target registry
```

---

## 9. Gate 2：统一 Runtime Source 扫描

### 目标

Scan 支持从 Claude Code / Codex / Custom 来源发现资产，但导入结果仍进入同一个资产中心。

### 必做

扫描来源：

#### Claude Code

```text
~/.claude/skills
<project>/.claude/skills
~/.claude/commands
<project>/.claude/commands
~/.claude.json
<project>/.mcp.json
```

#### Codex

```text
~/.agents/skills
<project>/.agents/skills
~/.codex/config.toml
<project>/.codex/config.toml
```

#### Custom

支持用户明确选择的：

- skill directory
- command directory
- MCP JSON/TOML config if adapter is known

### 输出

每个发现项必须包含：

```text
sourceId
provider
sourcePath/configPath
assetKind
assetName
sourceFormat
scope
isManaged
isSymlink
symlinkTarget
warnings
eligibleImport
eligibleAdopt
```

### 重要要求

- 没有显式 projectPath 时，不要用 Tauri current_dir 猜项目。
- 没有选中项目时，只扫 user/global 来源。
- 不扫描整盘。
- 所有 project scan 必须由用户选中 project 或明确输入路径触发。

### 测试

- fake HOME Claude sources。
- fake HOME Codex sources。
- project source 需要显式 projectPath。
- 无 projectPath 不扫描随机 current_dir。
- damaged JSON/TOML 返回 warning，不 fatal。

### Commit

```text
feat(desktop): unify runtime source scanning
```

---

## 10. Gate 3：Canonical Import

### 目标

任何来源的资产都导入到统一资产中心。

### 必做

Import 支持：

#### Skill

```text
Claude .claude/skills/<name>
Codex .agents/skills/<name>
Custom skill dir
→ assets/skills/<name>
```

#### Command

```text
Claude .claude/commands/<name>.md
Custom command file/dir
→ assets/commands/<name>.md
```

#### MCP

```text
Claude JSON mcpServers.<name>
Codex TOML [mcp_servers.<name>]
→ assets/mcps/<name>.json
```

Import 只读取选中的 live config entry，写入 canonical definition，并记录来源 target 的机器本地启用状态。
Import 不写回来源 live config，也不向 Claude/Codex 的其他 live config 自动分发。

当前产品尚未投入使用且不存在需要兼容的真实 MCP asset，因此直接采用该文件内容结构，不设计 `<name>/canonical.json` 迁移流程或双格式兼容期。

MCP canonical definition 需要能表达：

- name
- type (`stdio` / `http` / `sse`, omitted means `stdio`)
- command
- args
- url
- env
- headers
- cwd
- provider-specific extensions, including Codex env vars / HTTP env headers / timeouts / enabled tools / disabled tools when available

Import adapter 必须把 Claude/Codex 的目标字段转换为上述 canonical model。不能无损转换的字段保存在对应 `providerExtensions`，并在挂载到不同 provider 时显示 warning 或 blocked。

MCP canonical definition 不保存 target enable flag。target binding 记录在机器本地 registry / `mounts.yaml`，不参与 Git 同步。

MCP canonical storage 必须直接使用文件，不使用 SQLite、数据库表或 DAO。

### 冲突判定

Asset identity 只使用 `asset kind + name`。

- Skill/Command 与资产中心同名时直接标记 conflict，不做内容相等判断，也不根据 hash 自动跳过或合并。
- 已由本应用管理并挂回 runtime 的 symlink/junction 通过 registry 和链接目标识别为 managed source，不作为新冲突。
- Skill/Command UI 可以显示原文或文本 diff 供用户判断，但比较结果不参与自动 resolution。
- MCP 从 Claude JSON 或 Codex TOML 转换为 canonical JSON 后做结构化比较；字段顺序和格式化空格不同不算差异。
- MCP canonical 相同视为同一配置；canonical 不同时必须展示资产中心 JSON、candidate JSON 和可展开的原始 source。
- 每个 conflict 必须显式选择 `skip`、`overwrite` 或 `rename`，禁止自动 rename。
- `skip` 不修改 canonical asset 或 target binding。
- `rename` 必须使用新的安全名称创建资产，且不继承原资产 binding。
- `overwrite` 先备份再替换 canonical asset。
- Skill/Command overwrite 通过现有 link 自然生效；MCP overwrite 不自动写 live config，而是将已启用 targets 标记为 `outOfSync`，等待用户显式 Sync。

### 必须保留

- Import 不修改原位置。
- Import 可 planOnly。
- Import apply 先 backup 目标冲突。
- Import 不触碰未选择资产。
- Import 不触碰真实 HOME 测试。

### 测试

- Claude Skill import。
- Codex Skill import。
- Claude MCP JSON import。
- Codex MCP TOML import。
- Import 后 source 不变。
- planOnly 不写入。
- name/path traversal 被拒绝。
- duplicate conflict 可 skip/overwrite/rename。
- Skill/Command same-name conflict does not auto-resolve by content/hash。
- managed symlink/junction source is not reported as a duplicate。
- MCP canonical structural equality ignores key order/formatting。
- different MCP canonical JSON displays both sides and requires resolution。
- MCP overwrite marks enabled targets `outOfSync` without writing live config。

### Commit

```text
feat(desktop): import runtime sources into canonical asset center
```

---

## 11. Gate 4：Mount Preview 基于 Target

### 目标

Mount preview 不再只是 provider 逻辑，而是基于明确 MountTarget 和 adapter 生成计划。

### 必做

Preview input 改成或兼容：

```ts
type PreviewMountInput = {
  assetId: string;
  target: MountTarget;
};
```

Preview output 必须包含：

```text
compatibility
adapter
planned effects
target path/config path
backup requirement
warnings
canApply
unsupported reason
```

### 各类型 Preview

#### Skill

```text
asset center skill
→ target skill dir/<name> symlink
```

#### Command

```text
asset center command
→ Claude-compatible command target file symlink
```

#### MCP Claude JSON

```text
canonical MCP
→ JSON patch preview for mcpServers.<name>
```

#### MCP Codex TOML

```text
canonical MCP
→ TOML patch preview for [mcp_servers.<name>]
```

### 不兼容

- Command + Codex target 必须 blocked。
- Skill + MCP target 必须 blocked。
- MCP + skill target 必须 blocked。
- uninitialized provider user target 必须 blocked。

### 测试

- compatibility matrix。
- unsupported combinations。
- Claude skill preview。
- Codex skill preview。
- Claude MCP JSON preview。
- Claude user/local/project scope preview。
- Codex MCP TOML preview。
- Codex user/project scope preview。
- plan preview 不写入。

### Commit

```text
feat(desktop): preview mounts through target adapters
```

---

## 12. Gate 5：Mount Apply 基于 Target

### 目标

实现统一 mount apply，让同一份资产能挂到任意兼容目标。

### 必做

#### Skill apply

支持：

- Claude user skill dir
- Claude project skill dir
- Codex user skill dir
- Codex project skill dir
- custom skill directory

动作：

- backup existing target
- macOS/Linux replace target with symlink to asset center skill
- Windows replace target with directory junction to asset center skill
- reject unsafe path
- reject symlink/junction escape

#### Command apply

支持：

- Claude user commands
- Claude project commands
- custom command directory

不支持：

- Codex command

动作：

- backup existing command file
- replace with file symlink to asset center command file
- Windows permission failure is blocked with Developer Mode guidance
- do not fall back to copy or hardlink

#### MCP apply

支持：

- Claude user/local/project JSON config patch
- Codex user/project TOML config patch

动作：

- backup config file
- replace only selected server entry
- preserve unrelated entries
- Claude renderer only patches the selected scope's `mcpServers` container
- Codex renderer uses `toml_edit` and only patches `[mcp_servers]`
- preserve TOML comments and unrelated configuration
- map canonical `headers` to Codex `http_headers`
- remove legacy `[mcp.servers.<name>]` when deleting a Codex server
- disabling a target removes only that server from that target
- deleting an MCP asset removes it from every previously enabled target before deleting canonical storage
- do not symlink the whole Claude JSON or Codex TOML config
- do not manage OAuth token

所有 apply 共同要求：

- acquire ordered path locks
- validate preview expiry, input digest and file fingerprints inside the lock
- return `stalePreview` before writing when any dependency changed
- backup after validation and before mutation
- write structured files atomically
- use operation journal and rollback for multi-target operations

### 测试

- fake HOME Skill mount to Claude and Codex。
- fake project Skill mount to Claude and Codex。
- Command mount to Claude。
- Command to Codex rejected。
- MCP mount to Claude JSON。
- MCP mount to Claude local project entry。
- MCP mount to Claude project `.mcp.json`。
- MCP mount to Codex TOML。
- MCP mount to Codex project `.codex/config.toml`。
- unrelated MCP entries preserved。
- TOML comments and unrelated fields preserved。
- MCP import does not trigger reverse synchronization。
- target disable precisely removes one server。
- MCP delete cleans all previously enabled targets。
- mounted Skill/Command cannot be directly deleted。
- unmount only removes a link that still targets the canonical asset。
- user-replaced runtime content blocks unmount/delete。
- delete rollback prevents broken links and partial canonical deletion。
- Windows command wrapping is correct。
- Windows Skill uses directory junction。
- Windows Command uses file symlink and reports missing permission。
- Windows mount never silently falls back to copy or hardlink。
- uninitialized Claude/Codex user target is blocked without creating config。
- initialized provider may create a missing MCP config only after explicit preview。
- authorized project/custom target creation is listed in preview。
- backup generated。
- planOnly writes nothing。
- backup history records the generated backup。

### Commit

```text
feat(desktop): apply mounts through target adapters
```

---

## 13. Gate 6：Import and Adopt 组合流程

### 目标

App 端提供“导入”和“导入并接管”两个明确动作。

### UI 要求

Scan / Import 页面显示：

```text
[导入]
[导入并接管]
```

文案：

```text
导入：仅复制到资产中心，不改变当前生效位置。
导入并接管：导入后备份原位置，并挂载资产中心版本回原位置。
```

### 后端/编排

必须提供明确的后端命令：

```text
preview_adopt
adopt_apply
```

前端不得通过依次调用 import、backup、mount 来编排 Adopt。
Tauri command 只负责 DTO/进程状态，完整事务由共享 Rust core 执行。

Contract：

```ts
type PreviewAdoptInput = {
  sourceIds: string[];
};

type AdoptPreview = {
  importPlan;
  mountPlan;
  backupPlan;
  warnings;
  canApply;
};
```

### Apply

```text
import
→ backup original source
→ mount to original source location
→ refresh assets
→ refresh scan result
→ refresh mounts
```

`adopt_apply` 使用同一 operation journal、ordered locks、stale-preview 校验和 rollback 处理全部步骤。

### 安全

- 必须有 previewId。
- 必须展示 Preview 和明确确认按钮。
- planOnly 不写入。
- 任一环节失败必须回滚已完成步骤。
- 无法完整回滚时返回 partial failure、保留 operation journal，并展示人工恢复信息。
- 不得把部分成功作为整体成功。
- 不能留下半接管状态而无备份记录。
- 所有写入必须在安全边界内。

### 测试

- Claude Skill import and adopt。
- Codex Skill import and adopt。
- Claude Command import and adopt。
- Claude MCP import and adopt。
- Codex MCP import and adopt。
- planOnly no-write。
- source replaced only after successful import。
- failure reporting。

### Commit

```text
feat(desktop): add import and adopt workflow
```

---

## 14. Gate 7：GUI 整体收敛

### 目标

让 App 端能被用户顺畅人工验收。

### 必做页面

#### Dashboard

显示真实数据：

- asset counts
- mounted target counts
- backup counts
- git status
- last operations if available

无数据时显示空状态。

#### Asset Center

显示 canonical assets。
不要按 Provider 拆成不同资产池。
显示 `invalid` 和 `unregistered` 诊断状态，且这两类资产不能直接 mount。

- Skill/Command 提供只读内容预览和外部打开入口。
- MCP 提供 canonical spec 新增/编辑；保存后只标记现有 targets `outOfSync`，由用户显式 Sync。
可以有筛选：

- asset kind
- mounted/unmounted
- target compatibility

#### Scan / Import

支持：

- Claude sources
- Codex sources
- custom sources
- import
- import and adopt

#### Mount Manager

支持：

- 选择 canonical asset
- 选择 compatible target
- preview
- confirm
- apply
- refresh
- unmount one target
- delete asset through explicit unmount-all-and-delete preview

#### Conflict Resolver

支持所有 target adapter 的冲突展示。
如果某类 conflict 不能自动解决，必须明确说明。

Skill/Command 同名冲突由用户查看内容后选择 skip/overwrite/rename。
MCP 冲突同时展示 existing/candidate canonical JSON，并允许展开原始 Claude JSON 或 Codex TOML。

#### Backup History

能区分并查看：

- Git 可同步的 portable canonical backup。
- import/mount/adopt/MCP patch 产生的 machine-local runtime backup。

只读展示备份类型、适用设备、创建时间、manifest、受影响路径和敏感配置风险。
提供“在文件管理器中显示”和手动 Restore 教程，不提供应用内自动恢复按钮或写命令。
显示 backup 数量、总大小、最旧时间和容量提醒；只允许用户手动删除未被 operation journal 引用的 backup。

#### Settings

能配置：

- default scan roots
- default Claude user paths
- default Codex user paths
- project scan roots
- Git remote/branch
- UI display preferences

只读展示：

- fixed asset center root: `~/.my-agent-assets`

### 清理

- 不显示假数据。
- 不显示无效按钮。
- 不显示 Codex Command。
- 不显示 AGENTS.md asset。

### 测试

- 所有页面空状态。
- 所有页面真实数据状态。
- Visual QA 0 severe。
- 关键流程 UI tests。

### Commit

```text
feat(desktop): finalize unified asset management UI
```

---

## 15. Gate 8：CLI 接入共享 Core

### 目标

CLI 与 GUI 调用同一个 `crates/core` 业务实现。此 Gate 只完成命令入口、交互和输出收敛，不复制或重新实现业务逻辑。

### 必做命令

保留现有命令，必要时新增：

```text
maa init
maa scan
maa import <source-id | asset selector>
maa target list
maa target add <target-kind> --project <path> | --path <path>
maa target remove <target-id>
maa mount <asset-id> --target <target-id> --apply
maa unmount <asset-id> --target <target-id> --apply
maa remove <asset-id> --apply
maa adopt <source-id | asset selector> --apply
maa list
maa status
maa doctor
maa sync pull|push
```

`--project` / `--path` 只允许用于显式 `target add`，并经过规范化路径 Preview 和授权确认。
`mount/unmount/remove/adopt apply` 不接受任意前端/CLI 写入路径，只引用已登记的 `targetId`。

### 语义

```text
scan = 发现
import = 导入资产中心，不改变生效位置
mount = 挂载资产中心版本到目标
adopt = import + mount back to source
```

### 必须支持

- Claude Skill
- Codex Skill
- Claude Command
- Claude MCP JSON
- Codex MCP TOML

### 必须拒绝

- Command to Codex

### 测试

- CLI fake HOME full flow。
- CLI help 文案清晰。
- CLI plan/apply 行为一致。
- CLI 与 GUI 调用同一核心逻辑。
- 静态检查或结构审查确认 CLI/Tauri 没有重复文件操作和 provider adapter。

### Commit

```text
feat(cli): align commands with canonical asset workflow
```

---

## 16. Gate 9：同步与跨设备验证

### 目标

保证资产中心一份数据可以通过 Git 跨设备同步。

### 必做

- 确认 canonical asset center 中保存的是可同步内容。
- Git 只 stage 同步白名单，不执行无约束的 `git add .`。
- 同步 `backups/portable/`，忽略 `backups/local/`。
- Push 只允许经过 API 实时验证的 GitHub Private repository。
- visibility 未明确返回 `PRIVATE` 时禁止 Push。
- Pull 只允许 clean worktree 上的 fast-forward。
- Push 禁止 force，远端领先或分叉时必须阻止。
- 不自动 stash、merge、rebase 或 reset。
- Scan/Import/Mount/Adopt 不触发隐式 Git Sync。
- 不把本机 runtime symlink 目标当成跨设备状态硬编码。
- mount records 可以记录目标，但要明确是 machine-local runtime binding。
- Git sync 只同步资产中心和必要 manifest，不同步用户机器绝对 runtime 状态，除非文档明确说明。
- 新设备 pull 后不自动 mount。
- 远端资产删除后，本机 binding 显示 orphaned 并等待用户处理。

### 测试

模拟 A/B 设备：

```text
A fake HOME:
scan → import → adopt → sync push

B fake HOME:
sync pull → list assets → mount selected assets to B runtime paths
```

验证：

- B 有同一份资产。
- B 有同一份 portable backups。
- B 不继承 A 的绝对 runtime mount。
- B 可以自由选择目标挂载。

### Commit

```text
test(desktop): verify cross-device asset center sync semantics
```

---

## 17. Gate 10：最终安全加固复检

### 目标

对所有写入路径做最终复检。

### 必检

- assetId validation
- target path validation
- path traversal
- symlink escape
- junction escape
- Windows junction and file symlink capability detection
- no copy/hardlink fallback for Windows mount
- TOML patch safety
- JSON patch safety
- backup before replace
- planOnly no-write
- explicit confirmation after preview
- previewId binding
- preview expiry
- input digest binding
- SHA-256 stale fingerprint validation
- ordered cross-process path locks
- lock-inside revalidation
- same-directory atomic write and fsync
- multi-target operation journal
- rollback and partial-failure recovery
- startup recovery of incomplete operation journals
- new writes blocked until interrupted operation is recovered
- GitHub Private visibility check before every push
- public/internal/unknown/unverifiable remote push rejection
- remote identity change invalidates sync preview
- MCP values and Git credentials are redacted from logs
- no telemetry/crash analytics
- diagnostic export defaults exclude assets/live configs/backups
- fake HOME isolation
- no accidental writes outside HOME/project/custom approved root
- apply accepts registered targetId instead of arbitrary frontend path
- target registry is local-only and file-based
- project/custom target explicit authorization
- registry path/link change invalidates preview

### 测试

增加缺失测试，不要只靠人工检查。

### Commit

```text
test(desktop): verify final write safety boundaries
```

---

## 18. Gate 11：macOS Beta 与跨平台 V1 Stable

### 目标

先交付可独立安装验收的 macOS Beta，再完成 macOS + Windows 的跨平台 V1 Stable。

### 必做

- `npm run typecheck`
- `npm test`
- `npm run build:renderer`
- `npm run qa:visual`
- `cargo fmt --all -- --check`
- `cargo test -p my-agent-assets-desktop`
- `cargo test -p my-agent-assets-cli` if available
- Tauri dev smoke
- Tauri production build

macOS Beta 必须：

- 产出可安装的 `.app` 和 `.dmg`
- 没有 Apple Developer 证书时允许 ad-hoc 签名和未公证构建
- 文档明确说明首次启动可能需要用户手动允许
- 完成 macOS 安装、升级、启动、窗口与核心业务人工验收
- 可以在 Windows 完成前独立发布
- 明确标记为 macOS Beta，不宣称跨平台 V1 已完成

跨平台 V1 Stable 必须：

- macOS Beta 验收通过
- macOS 正式发布使用 Developer ID 签名并完成 Apple notarization
- 通过 Windows CI runner 产出 `.msi` 或 NSIS `.exe`
- Windows 正式发布使用代码签名；无证书构建只能标记为测试包，不能标记 Stable
- 完成 Windows 10/11 至少一次真实安装、升级、启动和核心业务人工验收
- 验证原生 Windows titlebar、无 28px 顶部空白、盘符/路径分隔符和跨卷行为
- 验证 junction/file symlink capability、Developer Mode 缺失提示及禁止 copy/hardlink fallback
- 验证 Windows Claude JSON 与 Codex TOML patch
- 验证安装、升级和卸载不删除 `~/.my-agent-assets`
- 完成基础键盘、可读名称、Focus、非纯颜色和 DPI/字体缩放验收

Windows 未完成时只阻止 V1 Stable，不阻止 macOS Beta 发布。

V1 不实现自动更新器：

- 不配置 updater endpoint。
- 不发起后台更新网络请求。
- 用户手动下载并安装新版本。
- 后续自动更新能力需要单独设计签名、发布源、完整性校验和回滚策略。

### 文档

创建或更新：

```text
docs/manual-acceptance-checklist.md
docs/final-beta-readiness.md
```

Manual checklist 必须覆盖：

1. 安装 App。
2. 空 HOME 打开。
3. 扫描 Claude assets。
4. 扫描 Codex assets。
5. 导入 Skill。
6. 导入并接管 Skill。
7. 同一 Skill 挂载到 Claude user target。
8. 同一 Skill 挂载到 Codex user target。
9. 同一 Skill 挂载到项目 target。
10. 导入 Command。
11. Command 挂载到 Claude commands。
12. Command to Codex 被拒绝。
13. 导入 Claude MCP。
14. 导入 Codex MCP。
15. 同一 MCP 挂载到 Claude JSON。
16. 同一 MCP 挂载到 Codex TOML。
17. 冲突处理。
18. 查看备份历史、定位备份文件并核对手动 Restore 教程。
19. Git sync。
20. 重启 App 后数据仍正确。
21. 不出现假数据。
22. 不出现无效按钮。
23. Logo 正常。
24. macOS/Windows 窗口行为不回归。

### Commit

```text
docs(desktop): prepare final manual acceptance build
```

---

## 19. 全局验证命令

每个 Gate 结束尽量运行：

```bash
cd apps/desktop && npm run typecheck
cd apps/desktop && npm test
cd apps/desktop && npm run build:renderer
cargo fmt --all -- --check
cargo test -p my-agent-assets-desktop
```

UI 或布局相关 Gate 额外运行：

```bash
cd apps/desktop && npm run qa:visual
```

CLI 相关 Gate 额外运行：

```bash
cargo test -p my-agent-assets-cli
cargo run -p my-agent-assets-cli --bin maa -- --help
```

最终 Gate 运行全部可用验证。

---

## 20. 偏差自我纠正规则

如果发现已经实现的代码偏向以下方向，必须立即纠正：

```text
Provider 分资产中心
Codex-only asset pool
Claude-only canonical format
Command to Codex
AGENTS.md asset management
fake data fallback
clickable no-op controls
runtime current_dir guessed as project
tests touching real HOME
planOnly writes files
MCP patch replacing unrelated entries
mount target incompatible but still可执行
```

纠正时优先修改模型、contract、测试，再修 UI。

---

## 21. 进度记录要求

实施过程中持续更新：

```text
docs/implementation-progress.md
```

格式：

```text
## Workstream / Gate N: <name>

Status:
- completed / blocked

Commits:
- optional until final commit consolidation

Validation:
- command: result

Implemented:
- ...

Not implemented:
- ...

Risks:
- ...

Next:
- Gate N+1
```

---

## 22. 最终交付标准

最终完成时，必须满足：

```text
用户打开 App 不看到假数据。
用户可以扫描 Claude 和 Codex runtime sources。
用户可以把扫描到的 Skill/Command/MCP 导入同一个资产中心。
用户可以把同一个 Skill 挂载到 Claude 和 Codex 目标。
用户可以把同一个 MCP 挂载到 Claude JSON 和 Codex TOML 目标。
Command 只支持 Claude-compatible targets。
用户可以执行导入并接管。
用户可以查看 portable/local 备份历史、定位备份文件并按照教程手动恢复。
用户可以用 Git 同步资产中心。
CLI 和 GUI 语义一致。
所有写入都有 preview、明确确认、backup、planOnly no-write 测试；高风险操作有高亮风险说明。
最终有可安装/可运行构建。
最终有人工验收清单。
```

---

## 23. 最终报告

完成全部 Gate 后，输出最终报告：

```text
Final status
Latest commit hash
All validation commands and results
Generated app/package path if available
Known limitations
Manual acceptance checklist path
Any unsupported environment notes
```

完成最终报告后停止。不要继续做 V1.1/V2.1/Post-V2 功能。
