# Zeta 架构文档索引

团队协作文档已按实现责任拆分：

- [Zeta Desktop 架构与协作边界](zeta-desktop-architecture.md)
- [Zeta CLI 架构与协作边界](zeta-cli-architecture.md)
- [zeta-rs 产品内核与对外层](zeta-rs-architecture.md)
- [Zeta App Server API v1（accepted）](zeta-app-server-api-v1.md)
- [Zeta API 接口文档规范](zeta-api-interface-requirements.md)
- [Zeta API 接口文档模板](zeta-api-interface-template.md)

原始统一方案
[zeta-code-architecture-codex-style-v2.md](zeta-code-architecture-codex-style-v2.md)
保留为历史设计依据；发生冲突时，以拆分后的责任文档和已接受的 API 契约为准。

Implemented foundations:

- `zeta-rs` Cargo workspace with explicit product crates;
- one App Server product contract shared by the in-process CLI client and Desktop stdio client;
- an accepted App Server API v1 baseline for Desktop and CLI development;
- stable internal protocol IDs and events;
- Core Thread/Turn state transitions with durable-event-before-state ordering;
- append-only rollout log, SQLite projection rebuilding, and exclusive writer leases;
- a single sandboxed process execution boundary;
- CLI `ask` and `exec` vertical slices;
- stdio JSON-RPC handshake, `thread/start`, `thread/read`, `thread/list`, `turn/start`, and
  `turn/interrupt`, with a state-root-scoped durable idempotency ledger;
- Rust-owned protocol generation for Desktop TypeScript and JSON Schema;
- Electron Main / Preload / Renderer baseline that starts the packaged app-server, completes
  initialize before Ready, and exposes only typed Thread and Turn IPC methods;
- semantic Browser Capability ports in Rust and Electron Main target ownership with isolated
  WebContents.
