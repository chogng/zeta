# Zeta 架构文档索引

## 文档规范

- [Zeta 文档分层与写作规范](documentation-guidelines.md)：规定 crate README 与
  `docs/*.md` 的读者、信息所有权、状态表达、推荐结构和 review checklist。

## 权威基线

- [Zeta 长期架构](zeta-code-architecture-codex-style-v2.md)
- [zeta-rs 产品内核与对外层](zeta-rs-architecture.md)
- [`zeta-protocol` 架构与演进方案](protocol.md)
- [`zeta-core` 架构](core.md)：Core ownership、执行组件、durable boundary、目录与落地顺序。
- [`zeta-core` Context 与 ContextManager](core-context.md)：per-Thread context ownership、
  organization、budget、compaction 与恢复。
- [`zeta-core` 多 Agent 架构](core-multi-agent.md)：delegation、spawn、context inheritance、
  message/result、cancellation 与 Agent tree budget。
- [Zeta Auto Review](auto-review.md)：contextual risk review、policy authority、用户交互、
  durable execution boundary、evaluation 与演进方向。
- [`zeta-config` 架构与 Plugin/MCP/Skill 接入方案](config.md)：Config authority、scope
  resolution、领域 snapshot reconcile 和 runtime safe point。
- [`zeta-model-provider-config` 架构与演进方案](model-provider-config.md)：Provider
  definition、静态配置、schema、merge 和 normalization。
- [`zeta-model-provider` 架构与演进方案](model-provider.md)：Provider/model resolution、
  direct-provider credential、API profile 选择和运行时组合。
- [`zeta-login` 架构](login.md)：用户可见的 interactive login control plane 与 redacted account
  lifecycle。
- [`zeta-codex-app-server` 架构](codex-app-server.md)：上游 Codex App Server adapter、
  ChatGPT/Codex subscription login 与 runtime boundary。
- [`zeta-secrets` 架构](secrets.md)：Provider-neutral secret `load/store/delete`、backend 与
  persistence 安全边界。
- [`zeta-api` 架构与演进方案](zeta-api.md)：`endpoint / requests / sse` 协议编解码、缓存
  wire 语义和 Provider error。
- [`zeta-http-client` 设计](../zeta-rs/http-client/README.md)：共享 HTTP/WebSocket backend、
  proxy、TLS、redirect、timeout、连接池和安全 transport diagnostics。
- [`zeta-client` 架构与演进方案](zeta-client.md)：API operation retry、SSE/NDJSON framing、
  operation deadline 和 telemetry。
- [Zeta App Server API](zeta-app-server-api.md)
- [App Server Client 架构与演进方案](app-server-client.md)
- [Zeta Desktop 架构与协作边界](zeta-desktop-architecture.md)
- [Zeta CLI 架构与协作边界](zeta-cli-architecture.md)
- [TUI 架构与演进方案](tui.md)
- [Zeta API 接口文档规范](zeta-api-interface-requirements.md)
- [Zeta API 接口文档模板](zeta-api-interface-template.md)

## Proposed 演进

- [`zeta-exec` 架构与演进方案](exec.md)：无交互 Agent runner、机器输出、远程调度 worker
  与独立 remote execution plane。
- [Zeta Agent 执行架构与演进方案](zeta-agent-runtime-architecture.md)：跨 Core、App Server、
  provider 与 Tool 的异步执行演进和 Provider safe point。
- [`zeta-models-manager` 架构与演进方案](models-manager.md)：统一管理 provider 模型发现、
  缓存、字段级合并、筛选、解析和 catalog snapshot。
- [`zeta-tools` 架构与演进方案](tools.md)：共享工具类型、definition/binding/executor、
  MCP/dynamic adapter、tool search、Plugin discovery、code mode 与图片精度。
- [`zeta-mcp` 架构与演进方案](mcp.md)：MCP client session、transport、capability negotiation
  与 tools/resources/prompts adapter。
- [`zeta-mcp-server` 架构与演进方案](mcp-server.md)：将 Zeta Agent 暴露给外部 MCP Host，
  并通过 App Server 复用 canonical Session/Thread/Turn 执行路径。
- [`zeta-plugins` 架构与演进方案](plugins.md)：扩展包 manifest、安装、权限、activation
  snapshot、更新与回滚。
- [`zeta-skills` 架构与演进方案](skills.md)：Agent Skills 发现、选择、渐进加载、context
  layering 与文件安全。
- [`zeta-pdf` PDF 文档入库与演进](pdf.md)：PDFium 原生处理边界、持久导入、OCR、
  引用与 RAG 知识库的职责划分。

Proposed 文档不能覆盖当前 API 与已实现领域边界。当前处于开发阶段，发生冲突时直接修正
canonical contract 和全部调用方，不建立旧 API、旧 DTO 或旧 storage schema 的兼容层。

## 当前已实现基础

- [`zeta-protocol` canonical contract 基础](protocol.md#9-当前完成度)；
- SessionCoordinator、Session reducer、SessionStore 与可恢复 Thread create/fork saga；
- ThreadController、Thread reducer、ThreadStore 与 durable Turn/Item/tool lifecycle；
- Session 和每个 Thread 的独立逻辑 sequence 与 writer lease；
- 一个共享的 event-stream 物理引擎，Session/Thread 只保留 typed store adapter；
- `zeta-rollout` 统一本地 authority 的打开与恢复，`zeta-rollout-trace` 只读导出可分析 trace；
- typed CommandId receipt、payload conflict detection 与 response replay；
- Session-first App Server API、snapshot + durable gap subscription，以及
  `session/update` / `thread/update`；
- `zeta-mcp-server` stdio/Streamable HTTP initialize、Agent tool start/reply、
  principal-scoped durable continuation/replay、bounded progress、form interaction、cancel 和
  bounded final result；
- `zeta-rmcp-client` / `zeta-mcp` 外部 tools client，以及启动时 App Server/Core catalog、
  one-time approval、durable result vertical slice；
- Rust DTO 生成 JSON Schema/TypeScript/schema hash；
- CLI/TUI in-process client 与 Desktop JSONL client 共用同一 dispatcher；
- Electron supervisor、strict JSONL framing、typed peer、trusted IPC 与 generated renderer
  contract；
- Config authority、provider registry、Resource 和 sandbox 基础。
