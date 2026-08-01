# UI 审查修复验收规范（P0 / P1）

日期：2026-08-01
范围：`apps/desktop` 静态 GUI 页面与共享布局样式。
状态：P0 修复已完成；本机 Visual QA 已通过，Figma 同步因 MCP Starter 额度限制待后续补传。

## 1. 审查基线

本规范基于两类证据：

1. 用户提供的生产态截图：Dashboard 空态、Skills 原生下拉展开态、Backup History 空态。
2. 正确代码分支重新生成的 Visual QA：13 个页面、macOS/Windows、`1440×900` 与 `1180×760`。

本轮代码与截图基线：

| 项目 | 值 |
| --- | --- |
| 分支 | `codex/ui-asset-edition` |
| 提交 | `ea00227` |
| Visual QA 生成时间 | `2026-08-01T16:25:17.972Z` |
| 自动检查 | 68 张截图；`severeCount=0`；`warningCount=0` |
| 已确认快照 | `docs/ui-audit/2026-08-01-correct-branch/` |

三张用于 Figma“当前实现”对照的快照均已与本轮 Visual QA 原图校验一致：

| 页面 | 文件 | SHA-256 |
| --- | --- | --- |
| Dashboard | `01-dashboard-1180x760-macos.png` | `4394ca61e5fb388bcdfa94e5cd085da6bf8e47be00add84c79a26bdd2f62660b` |
| Skills | `02-skills-1180x760-macos.png` | `7d7c75e11d288532204167d81424a1030b5e2264268527e18cbeecdc3e35fd13` |
| Backup History | `03-backups-1180x760-macos.png` | `c33ab14cf0e5eacf18e59798ca23b9a5fae0e1ecd09dfda1e4e097afef35d390` |

Visual QA 产物位于：

```text
apps/desktop/artifacts/visual-qa/
```

## 本轮修复记录（2026-08-02）

- P0-01：Dashboard 增加稳定内容宽度、等比分栏、中轴两侧固定内距；活动与项目空状态改为在完整面板内双向居中。
- P0-02：Skills、Commands、MCP Servers、Projects 统一使用可控状态菜单；支持键盘上下/Home/End、Esc、点击外部关闭，菜单不再依赖原生 `select` 弹出层。
- P0-03：Backup History 将标题、manifest 说明和状态拆成独立层级，避免摘要挤压标题和中文词组断行。
- P0-04：Dashboard、Sync、Scan、Backup 的状态徽标与系统检查项统一使用 success / warning / neutral / danger 语义色。
- P1：缩短紧凑宽度下的搜索提示；项目路径与资产详情字段保留完整值提示。

修复后 Visual QA：68 张截图，`severeCount=0`，`warningCount=0`；资产筛选菜单行为由组件测试覆盖。Figma 当前实现帧尚未替换：同步调用触发 Starter 计划 MCP 额度限制，未对 Figma 产生写入。

修复后用于 Figma 对照的快照位于：

```text
docs/ui-audit/2026-08-02-p0-fixed/
```

自动检查通过只证明没有检测到明显溢出、面板塌陷或低于最小字号的问题，不代表间距、对齐、原生弹出层、自然语言换行、语义颜色和可访问性已经通过。用户生产态截图与正确分支 demo 截图状态不同，二者在本规范中分别标注，不互相冒充当前实现证据。

验收必须覆盖以下矩阵：

| 平台 | 视口 | 必查内容 |
| --- | --- | --- |
| macOS | 1440×900 | 页面整体层级、分割线、面板对齐、空状态 |
| macOS | 1180×760 | 紧凑宽度下的工具栏、标题、副文案、长路径 |
| Windows | 1440×900 | 同上，且不出现 28px 顶部空白 |
| Windows | 1180×760 | 同上，且不出现原生控件裁切/重叠 |

## 2. P0：修复后才能进入下一轮视觉确认

### P0-01 Dashboard 结构与居中

**问题**

- 正确分支已经恢复统计区分割线与活动区中轴线，但活动区仍是无外框连续区域，左右列采用 `1.1fr / .9fr`，右列标题贴近中轴，而列表另加 24px 内边距，标题与内容起始线不一致。
- 主内容没有稳定的最大宽度/居中约束；宽屏下持续拉伸，标题、操作与列表内容的水平节奏不统一。
- 本轮正确分支 demo 为有数据状态，无法直接证明空状态双向居中；用户提供的生产态空状态截图仍显示视觉重心偏移，必须保留为专项回归项。

**验收**

- Dashboard 内容在主区域内有明确的内容宽度策略；宽屏下不会无限拉伸，窄屏下不被裁切。
- 两个活动面板拥有一致的左右内边距、顶部标题基线和底部边界。
- 中间分割线连续、可见，且位于两列视觉中心，不被标题或操作覆盖。
- 空状态图标、标题、说明在可用面板区域内水平和垂直居中；两个面板的视觉重心一致。
- “扫描资产”“管理项目”等操作与所属标题保持固定间距，不贴边、不跨越分割线。

**涉及页面**：Dashboard。

### P0-02 资产中心工具栏与原生下拉层

**问题**

- 正确分支在 `1180×760` 的关闭态下，搜索框与筛选框均位于容器内，未复现贴边或相互重叠；这一部分不再作为已复现缺陷。
- 用户提供的 macOS 原生 `select` 展开态会覆盖筛选框边框/焦点环；当前 Visual QA 只采集关闭态，尚不能关闭该问题。
- Skills、Commands、MCP Servers 复用同一个原生筛选实现，因此展开态、聚焦态和窄宽度回归需要按共享组件统一验证；Projects 仅在采用同类筛选控件时纳入。

**验收**

- 工具栏拥有统一的内边距 token；搜索、筛选和右侧统计均在容器边界内留出可见安全距离。
- 控件打开、聚焦、禁用、错误四种状态均不与边框或相邻内容重叠。
- 下拉层有明确的弹出定位、层级和阴影；不会被 `.asset-browser`、`.asset-inspector` 或页面滚动容器裁切。
- `1180×760` 下控件仍保持完整可读；不能通过缩小字号解决布局问题。
- Skills、Commands、MCP Servers、Projects 使用同一套工具栏规格。

**涉及页面**：Skills、Commands、MCP Servers、Projects。

### P0-03 标题、副文案与摘要的换行策略

**问题**

- Backup History 等页面的标题区使用左右 flex 子项，但没有为摘要、标题和技术文案定义优先级与换行规则。
- 正确分支在 `1180×760` 下仍将“Visual QA 示例数据”中的“据”挤到独立一行；左侧标题说明与右侧摘要争夺同一行宽度，问题已由本轮快照复现。
- 同类结构在 Conflict、Scan、Sync、Asset Detail、Project Detail、MCP 页面重复出现。

**验收**

- 标题、说明、摘要有明确的三段式布局：标题不被摘要挤压，摘要允许完整换行或在受控位置折行。
- 禁止在中文词组、状态词、版本号、文件名和英文 token 中间产生孤立断行。
- 长路径/技术值使用“单行省略 + 可查看完整值”或“受控换行”二选一，并在全局统一。
- `1180×760` 与 `1440×900` 的标题区高度稳定，不推动下方面板发生重叠或不可见。
- 所有页面副文案使用同一套文案宽度、行高和间距规则。

**涉及页面**：Backup History、Conflict、Scan、Mount、Sync、Asset Detail、Project Detail、MCP。

### P0-04 状态语义不能只靠绿色

**问题**

- “未初始化”“未连接”“读取失败”等非健康状态沿用绿色文本/徽标，用户会误判系统状态。
- Dashboard、系统状态和资产列表的状态颜色没有统一语义。
- 本轮正确分支 demo 主要覆盖健康态，不能据此确认未初始化、未知、失败和冲突状态已经满足语义规则；必须使用专门状态 fixture 或生产态复核。

**验收**

- `healthy/connected/ready` 使用绿色；`uninitialized/unknown/not-connected` 使用中性或警示色；`failed/error/conflict` 使用错误色。
- 文案、图标、颜色三者表达一致；颜色不是唯一的信息载体。
- Dashboard、状态卡、列表行、详情页和 Toast/错误提示使用同一状态 token。
- 通过键盘、灰度或色觉缺陷模拟仍能区分状态。

**涉及页面**：Dashboard、Skills、Commands、MCP、Projects、Scan、Mount、Conflict、Backup、Sync、Settings。

## 3. P1：P0 通过后进入的系统化改进

### P1-01 间距 token 与共享结构

- 页面、面板、标题区、工具栏、列表行、空状态和按钮间距全部引用统一 token；禁止继续新增无语义的 `10/12/14/17/18/20/24px` 变体。
- 抽取共享的 `SectionHeading`、`PanelHeader`、`AssetToolbar` 等结构，避免页面各自复制 flex 和 padding 规则。
- 保留当前冻结的 AppShell、Sidebar、macOS 28px drag area 和布局基础 token；本规范不授权修改窗口策略。

### P1-02 面板高度与局部滚动

- 固定高度只用于确有必要的工作区；内容较长时提供明确的局部滚动区域和可见滚动 affordance。
- `1180×760` 下详情、列表、检查项和操作区不被外层 `overflow:hidden` 静默裁切。
- 主操作区在视口内保持可达，不依赖滚动到页面最底部才能确认。

### P1-03 控件规格

- 主要输入、按钮、筛选器的高度统一为不小于 40px；辅助控件也要有一致的 focus ring 和 hit area。
- Settings、MCP 编辑字段、Apply confirmation 等页面不再混用 32px 与 40px 的视觉密度，除非明确标为紧凑表格控件。
- 键盘 Tab 顺序、Enter/Escape 行为、原生菜单定位需在 macOS 与 Windows 各验证一次。

### P1-04 空状态与下一步行动

- 空状态说明回答“当前是什么、为什么为空、下一步怎么做”。
- 能执行的下一步（扫描、导入、添加项目、创建资产）显示为明确 CTA；只读页面不伪造不可用按钮。
- 同类页面使用一致的图标、标题、说明、CTA 间距和对齐方式。

### P1-05 文案与诊断信息

- 用户可读文案统一中文或明确的中英术语表；禁止把 `asset-to-target binding`、`Target live config` 等内部术语直接暴露在主层级。
- 原始路径、系统错误号、诊断细节进入二级详情/复制诊断入口；页面主说明只保留可行动的安全提示。
- 页面标题、副标题、状态、按钮采用同一套句式和标点规则。

### P1-06 截断、对比度与辅助技术

- 被省略的路径、项目名和技术值必须可通过 tooltip、详情或复制操作查看完整内容。
- 12–13px 的辅助文本和低对比度灰色需要通过对比度检查；不得以缩小字号换取布局空间。
- 图标开关、状态徽标、下拉框和面板标题有可访问名称；焦点状态可见且不被 overflow 裁切。

## 4. 页面回归清单

| 页面组 | P0-01 | P0-02 | P0-03 | P0-04 | P1-01~06 |
| --- | --- | --- | --- | --- | --- |
| Dashboard | ✓ |  |  | ✓ | ✓ |
| Skills / Commands / MCP |  | ✓ | ✓（MCP） | ✓ | ✓ |
| Projects / Project Detail |  | ✓（列表） | ✓ | ✓ | ✓ |
| Scan / Mount / Conflict / Sync |  |  | ✓ | ✓ | ✓ |
| Backup History |  |  | ✓ | ✓ | ✓ |
| Settings |  |  | ✓ | ✓ | ✓ |

## 5. 修复后的验收顺序

1. 先用 `1180×760` 修复 Dashboard、资产工具栏和标题区的 P0 结构问题。
2. 在 `1440×900` 复核内容宽度、分割线、空状态重心和面板边界。
3. 将同一套共享 token/组件规则回归到所有页面组。
4. 运行 TypeScript、前端测试、renderer build、Rust 测试和 Visual QA。
5. 对四个视口/平台组合逐张对比参考截图与修复截图，并记录剩余 P1。

## 6. 当前明确不做

- 不修改 Tauri window config、AppShell 结构、macOS overlay 或 Windows titlebar 策略。
- 不改变业务数据模型、Tauri/Rust command contract、挂载/扫描/备份的写入安全流程。
- 不用 mock 数据掩盖空状态；修复只改变呈现、间距、层级、文案和可访问性。
