# Chat History Rollback

本文档描述 Chat 如何支持从任意历史用户请求编辑并重新发送，以及 Agent Host、Codex thread 和工作区状态如何协同回滚。

## 1. 摘要

任意点编辑不是单纯修改前端 transcript，而是一个跨层的截断和重放流程：

```text
用户选择历史请求
        |
        v
ChatModel 设置 checkpoint，并阻塞该请求之后的历史
        |
        v
用户编辑并提交
        |
        +--> 删除 Workbench ChatModel 中的旧尾部
        |
        +--> 通知 Agent Host 截断后端会话
        |        |
        |        +--> provider 将 host turn 映射为 backend turn
        |                 |
        |                 +--> Codex 调用 thread/rollback
        |
        +--> 必要时恢复工作区编辑快照
        |
        v
发送新的请求，接在保留的历史之后
```

该流程修改的是当前会话本身。它与 `Fork Conversation` 不同：Fork 会创建新的 chat/thread 并保留原会话。

## 2. 术语和边界

### 2.1 请求、turn 和 response

- **Request**：Workbench ChatModel 中的一条用户请求。
- **Response**：该请求对应的模型响应及工具调用结果。
- **Turn**：Agent Host 或具体 provider 保存的后端会话单位，通常包含用户消息、模型响应和工具结果。
- **Checkpoint**：Workbench 用来表示“从这里开始重写”的临时边界，不等同于持久化的完整会话副本。

Workbench request ID 与 provider 的 turn ID 必须能够对应。对于 Codex，fork 或恢复后可能产生新的后端 turn ID，因此需要维护 host turn ID 到 Codex turn ID 的映射。

### 2.2 三种回滚

```text
对话回滚       删除后端 thread 的历史尾部，使新请求不再看到旧尾部
工作区回滚     恢复 Agent 在旧请求中产生的文件编辑和相关快照
模型缓存回滚   由模型服务决定是否复用未变化的 prompt 前缀缓存
```

三者不是同一个操作。对话回滚不能撤销已经发生在外部系统中的副作用，例如已发送的网络请求、已经 push 的 commit 或已经发出的邮件。

## 3. 用户可见语义

假设会话为：

```text
M1 -> A1 -> M2 -> A2 -> M3 -> A3 -> M4 -> A4
```

用户编辑 `M3` 并提交 `M3'` 后，当前会话应变为：

```text
M1 -> A1 -> M2 -> A2 -> M3' -> A3'
```

其中：

- `M1`、`M2` 及其响应保持不变；
- 原来的 `M3`、`A3`、`M4`、`A4` 从当前路线中移除；
- 新请求使用原会话资源继续发送；
- 原路线不会自动保留为可切换分支。

如果用户需要同时保留两条路线，应使用 `Fork Conversation`。

## 4. Workbench 实现

### 4.1 进入编辑态

`ChatWidget.startEditing` / `clickedRequest` 完成以下工作：

1. 根据 request ID 找到任意历史请求。
2. 调用 `ChatModel.setCheckpoint(requestId)`。
3. 将 checkpoint 及其之后的请求标记为 blocked。
4. 将请求文本、附件和必要的动态变量填回输入框。
5. 记录当前输入状态，以便取消编辑时恢复。

这里的 blocked 是 UI/模型层的“后续请求应被移除”标记，不是模型服务的权限状态。

入口代码：

- [`chatWidget.ts`](./browser/widget/chatWidget.ts) 中的 `startEditing` 和 `clickedRequest`；
- [`chatModel.ts`](./common/model/chatModel.ts) 中的 `setCheckpoint`。

### 4.2 提交编辑

提交编辑时，`ChatWidget._acceptInput` 会：

1. 取消当前进行中的请求，或移除正在编辑的 pending request；
2. 结束编辑态；
3. 如果存在 checkpoint，删除 checkpoint 及之后的请求；
4. 发送新请求。

删除本地历史并不充分，因为 Agent Host 或 provider 可能仍保存着旧的后端 turn。因此发送新请求前后端必须同步截断。

相关代码：

- [`chatWidget.ts`](./browser/widget/chatWidget.ts) 中的 `_acceptInput`；
- [`chatServiceImpl.ts`](./common/chatService/chatServiceImpl.ts) 中的 `resendRequest`；
- [`chatModel.ts`](./common/model/chatModel.ts) 中的 `removeRequest`。

## 5. Agent Host 同步协议

### 5.1 为什么需要显式通知后端

如果只删除 Workbench 的请求：

```text
Workbench：M1 -> M2 -> M3'
后端：     M1 -> M2 -> M3 -> M4
```

新请求仍可能被后端拼接到旧 thread 的末尾，导致模型同时看到两条互相矛盾的路线。因此 Agent Host 在发送新 turn 前比较：

- 当前 ChatModel 中的请求顺序；
- provider protocol 中保存的 turn 顺序。

如果发现 ChatModel 已经比 protocol 少了历史 turn，则 dispatch `ChatTruncated`。

入口代码：

- [`agentHostSessionHandler.ts`](./browser/agentSessions/agentHost/agentHostSessionHandler.ts) 中发送新 turn 前的截断检测；
- [`actions.ts`](../../../platform/agentHost/common/state/protocol/channels-chat/actions.ts) 中的 `ChatTruncatedAction`；
- [`agentSideEffects.ts`](../../../platform/agentHost/node/agentSideEffects.ts) 中将 action 路由到 provider 的 `truncateChat`。

### 5.2 截断边界

编辑 `M3` 时，新请求尚未加入后端历史。Workbench 会找到仍然保留的前一个请求 `M2`，并发送：

```ts
{
	type: 'chat/truncated',
	turnId: M2.id,
}
```

含义是：保留 `M2` 以及之前的 turn，删除其后的所有 turn。

如果编辑的是第一条请求，则没有前置 turn，发送不带 `turnId` 的截断动作，表示清除该 chat 的旧历史。

## 6. Codex 实现

Codex 的 thread 回滚由 provider 负责，而不是由 Workbench 直接操作 Codex 数据库。

### 6.1 Codex 回滚流程

```text
ChatTruncated(turnId = M2)
        |
        v
AgentSideEffects
        |
        v
CodexAgent.truncateChat(chat, M2)
        |
        v
读取 Codex thread，解析 M2 对应的 Codex turn
        |
        v
计算 M2 之后的尾部 turn 数量
        |
        v
thread/rollback({ threadId, numTurns })
```

Codex adapter 需要处理两种 ID：

- Workbench/Agent Host 的 turn ID；
- Codex app-server 的 turn ID。

实时会话通常通过映射表转换；恢复的会话可以直接使用 Codex 持久化的 turn ID 作为 fallback。未知 ID 不应盲目按位置回滚，否则可能误删错误的会话尾部。

相关代码：

- [`codexAgent.ts`](../../../platform/agentHost/node/codex/codexAgent.ts) 中的 `truncateChat`；
- [`agentHostSessionHandler.ts`](./browser/agentSessions/agentHost/agentHostSessionHandler.ts) 中的 `ChatTruncated` dispatch；
- [`agentSideEffects.ts`](../../../platform/agentHost/node/agentSideEffects.ts) 中的 provider 调用。

### 6.2 回滚不等于撤销外部副作用

`thread/rollback` 主要修改 Codex thread 的对话历史。它不能保证撤销已经发生的任意外部副作用。

对于工作区文件，Agent Host 另外使用 editing session/snapshot controller，为每个历史 turn 建立 checkpoint，并在需要时恢复相关文件状态。文件快照恢复失败时，应向用户报告，而不能假设 thread 回滚已经恢复了工作区。

## 7. 缓存影响

任意点编辑会使被修改 turn 之后的 prompt 前缀发生变化，因此部分模型缓存会失效。

```text
原始：系统指令 + M1 + A1 + M2 + A2 + M3 + A3 + M4
重写：系统指令 + M1 + A1 + M2 + A2 + M3' + ...
                              ^^^^^^^^^^^^^^^^^ 需要重新计算
```

在按精确前缀缓存的服务中，未变化的系统指令、`M1/A1`、`M2/A2` 仍可能命中缓存；从 `M3` 开始的部分不能保证命中。缓存是否在 thread rollback 后保留，由具体模型服务决定，Workbench 不应向用户承诺缓存命中。

编辑位置越靠前，潜在的重新计算量越大。编辑最后一个请求通常比编辑早期请求便宜。

## 8. 与 Fork 的区别

### 8.1 编辑并重发

```text
原会话：M1 -> M2 -> M3 -> M4
编辑 M3 后：
原会话：M1 -> M2 -> M3'
```

- 复用同一个 session/chat/thread 身份；
- 对当前路线执行 destructive rollback；
- 原来的尾部不再作为当前历史显示；
- 可能复用未变化前缀的模型缓存；
- 不需要额外的会话入口。

### 8.2 Fork Conversation

```text
原会话：M1 -> M2 -> M3 -> M4
新会话：M1 -> M2 -> M3'
```

- 创建新的 session 或 peer chat；
- 原会话完整保留；
- 新会话从指定边界派生；
- 需要复制或派生会话元数据、turn ID、工具/工作区状态；
- 后端可能实现共享前缀，也可能重新构建上下文。

编辑适合“原思路走错了，改写当前路线”；Fork 适合“保留原思路，同时探索另一条路线”。

## 9. 一致性和失败处理

实现任意点回滚时必须满足以下不变量：

1. **边界一致**：UI、Agent Host 和 provider 使用同一个逻辑回滚边界。
2. **顺序一致**：新 turn 必须接在保留历史之后，不能接在旧尾部之后。
3. **ID 可解析**：恢复、Fork 或跨进程重连后仍能将 host turn 映射到 provider turn。
4. **未知边界安全失败**：找不到 turn ID 时不应猜测位置并删除数据。
5. **取消可恢复**：用户取消编辑时，不应删除原历史或触发后端回滚。
6. **并发受控**：回滚期间不能同时提交另一个 turn；进行中的请求需要先取消或排队。
7. **副作用明确**：thread 回滚、文件快照恢复和外部副作用撤销必须分别报告结果。
8. **重试幂等**：重复收到同一截断动作不能继续删除已经不存在的尾部。

建议记录以下诊断信息：session/chat 标识、host turn ID、provider turn ID、保留边界、删除的 turn 数、provider 回滚结果。日志中不应记录完整用户 prompt 或敏感工具参数。

## 10. 当前实现入口索引

| 能力 | 代码入口 |
| --- | --- |
| 历史请求进入编辑态 | [`chatWidget.ts`](./browser/widget/chatWidget.ts) |
| 设置/清除 checkpoint | [`chatModel.ts`](./common/model/chatModel.ts) |
| 删除请求并发送新请求 | [`chatWidget.ts`](./browser/widget/chatWidget.ts)、[`chatServiceImpl.ts`](./common/chatService/chatServiceImpl.ts) |
| 检测 UI 与 protocol 历史差异 | [`agentHostSessionHandler.ts`](./browser/agentSessions/agentHost/agentHostSessionHandler.ts) |
| `ChatTruncated` 协议动作 | [`actions.ts`](../../../platform/agentHost/common/state/protocol/channels-chat/actions.ts) |
| 将截断动作交给 provider | [`agentSideEffects.ts`](../../../platform/agentHost/node/agentSideEffects.ts) |
| Codex thread rollback | [`codexAgent.ts`](../../../platform/agentHost/node/codex/codexAgent.ts) |
| 文件编辑 checkpoint/snapshot | `workbench/contrib/chat/browser/chatEditing`、Agent Host snapshot controller |
| 独立分支会话 | [`chatForkActions.ts`](./browser/actions/chatForkActions.ts) |

## 11. 非目标

本文档不定义：

- 模型服务具体如何实现 prompt/KV cache；
- 如何撤销任意外部工具副作用；
- 如何把所有编辑路线永久保存为 DAG；
- 各 provider 必须使用相同的 thread 存储格式；
- UI 上具体显示哪些按钮或快捷键。

这些问题应分别由模型 provider、工具权限系统、会话持久化设计和具体 UI 规范负责。
