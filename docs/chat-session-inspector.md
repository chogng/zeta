# Chat Session Inspector：信息架构与分阶段设计

> 状态：Proposed。本文定义 Chat 内 Session Inspector 的产品语义、跨层所有权、
> Plan 投影边界、响应式交互和分阶段落地顺序；不表示这些能力已经完成。
> Desktop 的进程与权威状态边界以
> [`zeta-desktop-architecture.md`](zeta-desktop-architecture.md) 为准；
> Session、Thread、Turn 与 ThreadItem 的 canonical 语义以
> [`protocol.md`](protocol.md) 为准；
> App Server 的订阅和恢复契约以
> [`zeta-app-server-api.md`](zeta-app-server-api.md) 为准；
> Renderer 组件与 CSS 所有权以
> [`ui-styling-ownership.md`](ui-styling-ownership.md) 为准。

## 快速理解

Chat 右上角的辅助入口应打开 **Session Inspector**，而不是第二套 Session 导航。

产品层级固定为：

```text
Auxiliary Bar / Chat
├── Session tabs                 切换 Session
├── 当前 Session 的 ChatPane
│   └── 当前 Thread transcript
└── Session Inspector
    └── 当前 Thread 的 Plan、状态和辅助投影
```

第一阶段只实现 **Plan**，后续再按真实后端 contract 增加 Threads、Activity、Changes、
Context 和 Artifacts。无论增加多少功能，Inspector 都只跟随当前
`Session + Thread`，不得自己维护另一份 Session 选择状态。

关键决策如下：

| 问题 | 决策 |
| --- | --- |
| Inspector 的作用域 | 当前 `Session + Thread` |
| Session 导航 | 继续由 Chat 顶部 tabs 和 History 入口拥有 |
| 第一项功能 | 当前 Thread 的 Plan |
| Plan 的权威 owner | Rust canonical Thread/Turn model |
| Renderer 的职责 | 可丢弃、可重建的 Plan 视图投影 |
| 第一阶段数据入口 | `ThreadUpdate::PlanUpdated` contract；production emitter 尚未接通 |
| 第一阶段恢复限制 | 重连或重新选择后无法从 snapshot 恢复结构化 Plan |
| 生产完整形态 | `thread/read` 必须返回可恢复的结构化 Plan |
| Inspector 对 Plan 的修改 | 第一阶段只读；不直接勾选或静默改变步骤 |
| 订阅策略 | Transcript 与 Inspector 共享已有 Thread projection，不建立第二条独立订阅 |
| 窄宽度行为 | 抽屉/覆盖层；不得把 Chat 正文压缩到不可用宽度 |
| 宽布局行为 | 空间足够时可并排显示 |

## 2. 要解决的问题

当前 Chat 已经同时出现三个容易混淆的导航概念：

- Workbench 的 Auxiliary Bar 承载整个 Chat；
- Chat title tabs 映射活动 Session；
- Chat 内还有一个名为 Agent Sidebar 的嵌套区域和开关。

如果嵌套区域再次按 Session 组织内容，用户无法判断顶部 tabs 和内部 Sidebar 谁拥有主导航，
关闭、切换和恢复的语义也会重复。另一方面，Agent 执行过程中确实存在适合持续可见、但不适合
反复插入 transcript 的辅助状态，Plan 是当前最明确的真实消费者。

本设计解决：

1. 固定 Session、Thread 和 Inspector 的导航层级；
2. 让 Plan 在当前工作期间持续可见；
3. 保持 Plan 权威状态与 Renderer 展示分离；
4. 防止第二条订阅、文本解析或本地持久化形成冲突状态；
5. 为后续辅助能力提供受控扩展方向。

## 3. 不包含的目标

Session Inspector 不负责：

- 展示或管理所有 Session 历史；
- 替代 Chat title tabs、History Quick Pick 或未来专用 Session 页面；
- 执行 Agent 规划、判断步骤是否完成或修改 canonical Plan；
- 从 Markdown、Agent 消息或 Tool Result 猜测结构化 Plan；
- 承载完整 Terminal、Browser、Settings 或 Editor；
- 仅凭 Git working tree 将文件变化归因到某个 Session；
- 在 Renderer 中持久化一份服务端无法验证的 Plan；
- 把 `ViewContainerLocation.AgentSidebar` 扩展成没有真实消费者的通用插件平台。

## 4. 当前实现状态

以下均为 Current，可从 Desktop 与生成协议代码验证：

| 能力 | 当前状态 | 边界 |
| --- | --- | --- |
| Chat 注册到 Auxiliary Bar | ✅ | Chat 是独立 View contribution |
| 一个活动 Session 对应一个 Chat tab 和保留的 `ChatPane` | ✅ | tab 只切 Session |
| `IWorkbenchSessionService` 统一保存当前 Session/Thread 选择 | ✅ | 不拥有 transcript |
| Chat title 提供 Agent Sidebar toggle | ✅ | 默认隐藏 |
| Chat 内创建嵌套 `SidebarPart` | ✅ | 当前没有可用的 Agent Sidebar 内容装配 |
| `ViewContainerLocation.AgentSidebar` | ✅ 扩展点 | 尚无真实 ViewContainer consumer |
| durable `ThreadItem::Plan` 类型与 reducer | ✅ | 只有 Markdown `text`；尚未发现 production producer |
| `PlanUpdate { explanation, steps }` | ✅ | 结构化步骤状态 |
| `PlanStepStatus` | ✅ | 只有 `pending`、`inProgress`、`completed` |
| `ThreadUpdate::PlanUpdated` 类型与 broker routing | ✅ | transient；尚未发现 production producer |
| `ChatPaneModel` 接收 `planUpdated` | 部分具备 | 当前分支明确忽略该 update |
| Chat transcript 渲染 Plan item | ✅ | 收到 item 时渲染 Markdown，不提供结构化步骤投影 |
| 结构化 Plan 从 `thread/read` 恢复 | ❌ | `Thread` / `Turn` snapshot 当前没有该字段 |

因此，“把 Plan 放进 Inspector”已有协议和 Renderer 切入点，但 P1 仍需接通真实
`planUpdated` producer。即使完成实时 vertical slice，也不能把它描述为跨重启、重连或重新
选择后可靠恢复。

## 5. 用户可见信息架构

### 5.1 导航归属

| 用户动作 | 唯一 owner | Inspector 的反应 |
| --- | --- | --- |
| 切换 Session | Chat title tabs / History | 跟随新 Session 的当前 Thread |
| 切换 Thread | Thread navigation | 跟随新 Thread |
| 显示或隐藏辅助内容 | Session Inspector toggle | 不改变 Session/Thread 选择 |
| 关闭 Session | Session tab | Inspector 释放该 Session 投影 |
| 更新 Plan | Agent runtime / canonical Thread | Inspector 只刷新展示 |

Session 不得被注册为 Inspector 内的 ViewContainer、ViewPane 或 tab。未来 Inspector 如果需要
内部导航，导航项必须是 Plan、Threads、Activity、Changes 等**功能视图**，而不是 Session
实例。

### 5.2 目标布局

Session Inspector 的目标信息布局如下。阶段标签表达实施顺序，不表示所有区块会在 P1
同时出现：

```text
Session Inspector
│
├── Plan                                      P1 / P2
│   ├── ✓ Understand terminal layout
│   ├── ● Implement tabs behavior
│   ├── ○ Add tests
│   └── ○ Verify UI
│
├── Threads                                   P3
│   ├── ● Main Thread
│   └── ○ Layout investigation
│
├── Activity                                  P3
│   ├── Turn: running
│   └── Waiting: none
│
├── Changes                                   P4
│   ├── M terminalViewPane.ts
│   ├── M terminalTabsLayout.ts
│   └── M terminal.css
│
├── Context                                   P4
│   ├── Workspace: zeta
│   ├── AGENTS.md
│   └── 3 referenced files
│
└── Artifacts                                 P4
    └── Generated outputs
```

布局规则：

- Plan 始终位于首位，是 P1 唯一必须实现的内容；
- Threads 只展示当前 Session 内的 Thread，不展示其他 Session；
- Activity 与 Plan 分开：Turn 等待、审批和运行状态不能伪装成 Plan step 状态；
- Changes 只有具备 Session/Thread attribution 后才能出现；
- Context 只展示 canonical context/attachment projection，不从 transcript 猜测；
- Artifacts 只有具备稳定 identity、lifecycle 和 open contract 后才能出现；
- 尚未达到对应阶段的区块不渲染，不用 disabled placeholder 提前占位；
- 多个区块同时存在时使用纵向 section；Plan 默认展开，其余区块允许折叠；
- 折叠状态是可丢弃的 Renderer UI 状态，不能写入 Session、Thread 或 Plan。

### 5.3 Plan 视图

Plan 默认显示在 Inspector 的第一个位置：

```text
PLAN

说明文本（可选）

✓ Understand terminal layout
● Implement tabs behavior
○ Add tests
○ Verify UI
```

展示规则：

- 保留 `PlanUpdate.steps` 的服务端顺序；
- `pending`、`inProgress`、`completed` 使用不同图标、文本和稳定状态 class；
- 不推断 `blocked` step；等待审批、用户输入或 capability 属于 Turn/Activity 状态；
- 不因为 UI 认为计划异常而重排、去重或自动修正步骤；
- `explanation` 为空时不保留空白占位；
- 第一个版本只读，不提供可勾选 checkbox；
- 没有结构化 Plan 时显示 `No active plan`，不得回退解析 Plan Markdown；
- Thread 切换、stream incarnation 变化、更新空洞或 reconnect 时，实时 Plan 必须清空，
  直到收到新 `planUpdated` 或未来从 durable snapshot 恢复。

当前 `PlanStep` 没有 step ID，也没有 transcript item correlation，因此第一阶段不支持：

- 点击步骤精确跳转到对应消息或 Tool Call；
- 对单个步骤做稳定的跨更新动画 identity；
- 用户直接修改单个步骤；
- 将重复文本步骤视为同一个步骤。

这些交互只有在 canonical contract 增加稳定 identity/correlation 后才能设计。

### 5.4 空、加载和错误状态

| 状态 | 展示 |
| --- | --- |
| 当前 Thread 正在加载 | 与 Chat 共用 projection loading 状态；显示轻量 loading |
| 已加载但没有 Plan | `No active plan` |
| 收到实时 Plan | 显示 explanation 和步骤 |
| Thread 切换 | 立即移除旧 Thread Plan，再绑定新投影 |
| reconnect / stream gap | 清除不可恢复的实时 Plan，等待 resync |
| Thread projection error | 显示与 Chat 一致的可恢复错误，不伪造旧 Plan |
| 没有活动 Session | `Start a new chat to view its plan` |

不得使用上一个 Session 或 Thread 的 Plan 填充 loading/empty 状态。

## 6. 响应式布局

当前 Chat 通常位于较窄的 Auxiliary Bar。现有固定 220px 的嵌套 Sidebar 会显著压缩 transcript，
因此目标布局不能始终并排。

| Chat 可用宽度 | Proposed 行为 |
| --- | --- |
| 小于 720px | Inspector 作为右侧抽屉覆盖 Chat body，默认隐藏 |
| 大于等于 720px | Inspector 可与 Chat body 并排，建议宽 280px |
| 无活动 Session | toggle 可保留，但打开后只显示 empty state |

布局应保证：

- Chat 正文并排时至少保留 360px；
- Inspector 建议最小宽度 240px；
- 抽屉打开后可通过 toggle、关闭按钮或 `Escape` 关闭；
- 抽屉不改变 Session/Thread 选择；
- focus 进入抽屉后关闭应返回触发按钮；
- 是否采用 split 或 drawer 由 Chat host 可用宽度决定，不写入 durable settings。

Inspector 是 Chat contribution 内的局部布局区域，不是新的 Workbench Part。是否复用基础
primitive 由实现决定，但不应继续用一个完整 `SidebarPart` 嵌套另一个 Workbench
Auxiliary Bar 来获得视觉效果。

## 7. 组件与数据所有权

### 7.1 跨层所有权

| 组件/层 | 拥有 | 明确不拥有 |
| --- | --- | --- |
| Rust Protocol / Core | Plan 结构、状态词汇、Turn 归属、durable 恢复语义 | Desktop 布局与展开状态 |
| App Server | snapshot、subscribe/update delivery、sequence 与 reconnect contract | Inspector 是否可见 |
| `IWorkbenchSessionService` | 当前 Session/Thread 选择和 Session topology projection | Thread transcript 与 Plan |
| Thread projection owner | 一个 Thread 的 snapshot、transient items、Plan live state 和订阅生命周期 | Session tabs 和 Inspector 布局 |
| `ChatViewPane` | Session tabs、当前 ChatPane、Inspector 外部布局和 active projection 绑定 | Plan 状态判断 |
| `SessionInspector` | 局部导航、empty/loading 呈现、focus 与响应式行为 | Session 选择和 App Server 调用 |
| Plan view/widget | Plan DOM、ARIA、步骤状态投影 | 修改 canonical Plan |

### 7.2 单一 Thread 订阅

当前每个保留的 `ChatPaneModel` 已经拥有选中 Thread 的 `thread.subscribe`、`thread.read`、
stream cursor 和 reconnect 处理。Inspector 不得再为同一 Thread 独立调用
`thread.subscribe`，否则两个消费者的 unsubscribe、cursor 和 reconnect 生命周期会相互干扰。

第一阶段应复用现有 projection：

```text
thread.subscribe / thread.read / thread update
  → ChatPaneModel（唯一订阅 owner）
  ├── transcript items
  └── live Plan projection
       → ChatPane / ChatViewPane
       → SessionInspector
       → Plan view
```

如果 Plan、Activity、Changes 等消费者增长到 Chat 私有 model 不再合适，应提取共享的
Thread projection service。提取前必须有至少两个真实消费者，并保持以下 contract：

- acquisition/release 不导致一个消费者释放另一个消费者仍在使用的订阅；
- 同一 Thread 只有一个 stream cursor authority；
- reconnect 与 gap recovery 统一执行；
- UI consumer 只能读取 projection，不直接写 canonical state。

不得为了让 Inspector 快速工作而把 `ChatPaneModel` 整体注册成全局 service，或让
`SessionInspector` 通过 DOM 查找当前 ChatPane。

## 8. Plan 的可靠性与持久性

### 8.1 当前契约

当前存在两种不同的 Plan 表示：

| 表示 | 结构 | Durability | 当前用途 |
| --- | --- | --- | --- |
| `ThreadItem::Plan` | `text: String` | durable | transcript 中的 Plan 文本 |
| `ThreadUpdate::PlanUpdated` | `PlanUpdate { explanation, steps }` | transient | 低延迟结构化进度 |

两者不能互相冒充。Renderer 解析 Markdown 会丢失状态、顺序语义和未来兼容性；把 transient
Plan 写入 local storage 又会产生服务端无法确认的陈旧状态。

### 8.2 计划中的持久化契约

生产完整版本应让 Plan 成为 Turn 的可恢复 readable state。推荐方向是：

```text
Turn {
  ...
  plan?: PlanUpdate
}
```

并由明确的 durable Thread event 更新该字段。具体 Rust variant 与迁移由 Protocol/Core
实现评审确定，但必须满足：

1. event 足以从空状态重建 `Turn.plan`；
2. `thread/read` 和 `thread/subscribe` snapshot 返回同一结构化 Plan；
3. `planUpdated` 可以继续作为低延迟 preview，但不能成为唯一恢复来源；
4. preview 被 committed state 覆盖，而不是覆盖 committed state；
5. Plan 归属 `TurnId`，不提升为 Session 级全局状态；
6. completed Turn 可以保留最终 Plan，供 Inspector 显示 `Last plan`；
7. 新活动 Turn 没有 Plan 时，不自动继承上一 Turn 的可编辑状态；
8. status 词汇的新增或约束由 Rust canonical contract 校验，Renderer 不自行扩展。

在该 contract 落地前，Desktop 文档、UI 和测试必须继续把 Plan Inspector 标为实时投影。

## 9. Renderer 组件设计

目标组件边界为：

```text
ChatViewPane
├── ChatTitleControl
├── active ChatPane
└── SessionInspector
    └── PlanView
```

`SessionInspector` 是 Chat contribution 拥有的 composed control，不是通用 base 组件。只有当
第二个领域出现相同的、与 Session 无关的交互 contract 后，才考虑下沉通用 primitive。

建议状态输入使用带语义的联合类型，而不是多个 bool：

```ts
type PlanPresentation =
  | { readonly phase: "loading" }
  | { readonly phase: "empty" }
  | { readonly phase: "ready"; readonly plan: PlanUpdate }
  | { readonly phase: "error"; readonly message: string };
```

这是 Proposed 示例，用于说明调用方可读性，不表示当前 API 已经存在。

### 9.1 Action 与上下文状态

用户可见名称应从泛化的 `Agent Sidebar` 收敛为 `Session Inspector`：

- `Show Session Inspector`
- `Hide Session Inspector`
- toolbar `aria-label` 保持 `Chat layout`

最终只允许一个 visible context state。若保留旧 command ID 用于兼容，它必须委托同一命令和
同一 context key，不得产生 Agent Sidebar 与 Session Inspector 两套可见状态。

### 9.2 CSS 所有权

| 样式 | Owner |
| --- | --- |
| Chat body 与 Inspector 的 split/drawer 几何 | `ChatViewPane` / Chat layout CSS |
| Inspector 内部 header、section 和 focus | `SessionInspector` |
| Plan step 排列、status class 与图标 | Plan view |
| toggle checked 状态 | Action → Button 的 `.checked` 状态投影 |
| 颜色和尺寸值 | semantic design token |

视觉 selector 使用稳定 class，例如 `.in-progress`、`.completed`；ARIA attribute 只表达
无障碍语义，不作为 CSS 状态 selector。Host CSS 不穿透修改共享 Button、TabList 或
ScrollableElement 的内部状态。

### 9.3 无障碍

- Inspector root 使用明确的 `aria-label="Session Inspector"`；
- Plan 使用有序列表语义；
- 每个步骤同时提供可读状态文本，不能只靠颜色或图标；
- `inProgress` 可使用 `aria-current="step"` 表达当前位置，但 CSS 仍选择状态 class；
- transcript 的 `role="log"` 不应因 Plan Inspector 更新而重复播报整个列表；
- drawer 模式必须有可靠的 focus return 和 `Escape` 行为；
- loading、empty 和 error 不使用持续刷新的 `aria-live="assertive"`。

## 10. 后续功能边界

Plan 之外的功能只有在数据 owner 明确后才能加入：

| 功能 | 适合度 | 前置条件 | 建议阶段 |
| --- | --- | --- | --- |
| Plan | ✅ | 当前 transient contract 已有；durable contract 后续补齐 | P1/P2 |
| Threads | ✅ | Session topology、Thread title/read 和选择交互统一 | P3 |
| Activity | ✅ | 从 Turn status 与 tool events 做只读投影 | P3 |
| Changes | 适合，但尚未完成 | 必须有 Session/Thread attribution，不能只读全局 Git diff | P4 |
| Context | 适合，但尚未完成 | canonical context/attachment projection | P4 |
| Artifacts | 适合，但尚未完成 | artifact identity、lifecycle 与 open contract | P4 |
| Session history | ❌ | 已由 tabs/History 拥有 | 不进入 Inspector |
| 完整 Terminal/Browser | ❌ | 属于现有 Workbench surface | 不进入 Inspector |

Threads 进入 Inspector 后仍是当前 Session 内的 Thread topology，不是第二份 Session history。
只有后端能证明变化与当前 Session/Thread 的关联时，变更（Changes）才能显示为“本次会话
修改”；否则只能叫“工作区变更”，并留在 SCM/Files 等已有接口面。

## 11. 被拒绝的替代方案

| 方案 | 判断 | 原因 |
| --- | --- | --- |
| Inspector 内再列所有 Session | ❌ | 与 Chat tabs/History 重复，产生双主导航 |
| 从最新 Plan Markdown 解析步骤 | ❌ | 非 canonical，状态和兼容性不可恢复 |
| 把实时 Plan 写入 Renderer local storage | ❌ | 形成无法由服务端确认的陈旧 authority |
| Inspector 自己订阅当前 Thread | ❌ | 重复 subscription、unsubscribe 和 cursor authority |
| 用 `blocked` 扩展前端 step 状态 | ❌ | 当前 protocol 没有该状态 |
| 用户直接勾选 Plan step | 尚不采用 | 当前没有修改 canonical Plan 的 command contract |
| 始终固定 220px 并排 | ❌ | Auxiliary Bar 中会使 Chat 正文不可用 |
| 永久嵌套完整 `SidebarPart` | 暂不采用 | Inspector 是 Chat 局部区域，尚无独立 Workbench composite 需求 |
| 为未来插件预建多级 ViewContainer | 暂不采用 | 当前只有 Plan 这个真实消费者 |

## 12. 分阶段实施

### P1：实时 Plan Inspector

目标：接通 `planUpdated` producer，并做正确但明确可丢失的实时展示。

实施范围：

1. 让 Agent execution vertical slice 产生真实 `PlanUpdate`，并通过现有 broker 发布
   `planUpdated`；
2. 将用户可见名称改为 Session Inspector；
3. 用 Chat-owned `SessionInspector` 替换空的嵌套 Sidebar host；
4. 让现有 Thread projection 保存当前 `PlanUpdate`；
5. 将 active ChatPane projection 绑定给 Inspector，不增加订阅；
6. Thread/session 切换和 reconnect/gap 时清除旧实时 Plan；
7. 实现 drawer/split 响应式布局；
8. 增加 producer、model、view、toggle、focus 和跨 Session 隔离测试。

完成条件：

- 真实 Agent Plan update 能从 runtime 到达 Desktop，而不依赖测试 fixture；
- 一个 Session 的 Plan 不会出现在另一个 Session/Thread；
- `planUpdated` 顺序和状态被原样显示；
- 隐藏 Inspector 不停止 Chat 的 Thread subscription；
- Inspector 打开或关闭不改变当前 Session/Thread；
- 重连后不显示无法确认的旧 Plan；
- 窄 Auxiliary Bar 中 Chat 正文不会被固定侧栏永久压缩；
- UI 不解析 transcript Markdown。

### P2：可恢复的结构化 Plan

目标：刷新、重连和重新选择 Thread 后仍能从 canonical snapshot 恢复 Plan。

实施范围：

1. Protocol/Core 接受 durable Turn Plan contract；
2. App Server schema 与生成 TypeScript 同步更新；
3. reducer/store/recovery 证明 event 可重建 `Turn.plan`；
4. `thread/read` / subscribe snapshot 返回结构化 Plan；
5. Desktop projection 先应用 snapshot，再接受 transient preview；
6. 增加 Rust contract/store/recovery 与 Desktop reconnect 测试。

完成条件：

- 丢弃所有 transient update 后，Inspector 仍能从 snapshot 显示最后 committed Plan；
- stream gap 不会让旧 preview 覆盖 committed Plan；
- App Server crash/reconnect 后 Plan 与 Thread snapshot 一致；
- completed Turn 的最终 Plan 显示语义明确。

### P3：Threads 与活动

目标：增加已有 canonical 数据能够支持的辅助投影。

- Threads 只展示当前 Session membership/lineage；
- Activity 展示当前 Turn status、等待类型和运行状态；
- 不复制 transcript 中完整 Tool Call/Result 内容；
- 不改变 Session tabs 的主导航职责。

### P4：变更、上下文与产物

这些能力必须分别先完成 attribution、identity 和 lifecycle contract。不能为了填充 Inspector
而从 DOM、Git 全局状态或聊天文本推断。

## 13. 测试与验收

### 13.1 P1 自动化测试

| 测试层 | 必须覆盖 |
| --- | --- |
| Projection unit | 接收正确 Thread 的 `planUpdated`、忽略其他 Thread、switch/reset/gap/reconnect |
| Chat view | toggle、active Session 切换、empty/loading/error、无第二条 subscribe |
| Plan view | explanation、三种 status、顺序、重复 step 文本、空 steps |
| Accessibility | label、ordered list、状态文本、focus return、Escape |
| Responsive policy | width breakpoint 对应 drawer/split，不依赖 JSDOM 实际布局猜测 |
| Regression | Session tabs、composer draft retention、archive、transcript scroll 不受影响 |

### 13.2 P2 契约测试

- durable Plan event serde/schema；
- reducer 从空状态重建 `Turn.plan`；
- store restart recovery；
- subscribe snapshot + committed gap；
- transient preview 与 committed Plan 的覆盖顺序；
- Desktop reconnect 和 stream cursor incarnation 变化。

### 13.3 人工验收场景

1. 在 Session A 运行一个包含 Plan 的任务并打开 Inspector；
2. 切换到没有 Plan 的 Session B，确认不泄漏 A 的内容；
3. 返回 A，P1 应按实时限制展示 empty 或新 update，P2 应从 snapshot 恢复；
4. 在窄 Auxiliary Bar 打开 Inspector，确认以 drawer 呈现；
5. 在未来 Editor/New Window 宽布局打开 Chat，确认可以 split；
6. 中断、失败、等待审批时，step status 不被 Renderer 擅自改写。

## 14. 长期不变量

无论实现如何演进，都必须保持：

1. Session tabs 是 Chat 的 Session 主导航，Inspector 不复制它；
2. Inspector 的内容始终绑定当前 `Session + Thread`；
3. Plan 语义和 durability 属于 Rust canonical model，Renderer 只是 projection；
4. transient update 可以全部丢失而不损坏 authoritative state；
5. 一个 Thread 只有一个客户端 subscription/cursor authority；
6. Inspector 可见性是可丢弃 UI 状态，不进入 Session/Thread canonical model；
7. 用户可见状态不能由 Markdown、DOM 或全局 Git 状态猜测；
8. 新增 Inspector 功能前必须先说明 owner、作用域、恢复语义和已有 surface 边界；
9. 窄布局不能为了显示辅助信息而永久破坏 Chat 的主要输入与 transcript；
10. UI 状态 class、ARIA 与 CSS owner 遵循 Renderer styling canonical contract。
