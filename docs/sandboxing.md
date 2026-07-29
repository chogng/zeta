# Zeta Sandbox 架构

## 1. 定位

Zeta 将 action review、sandbox policy、平台选择、命令构建和 OS enforcement 分开：

```text
Core ToolScheduler
  → zeta-policy
    → zeta-auto-review（需要时）
    → local Tool/command executor
      → zeta-sandboxing
      ├─ macOS Seatbelt backend
      ├─ zeta-linux-sandbox
      │    └─ zeta-bwrap
      └─ zeta-windows-sandbox
```

这里的 `SandboxManager` 只调度 sandbox backend：它验证 Workspace 相对路径、解析当前平台的
policy，并生成可执行的 host launch plan。它不负责 Tool 并行计划、approval、retry、
deterministic result ordering 或 durable Tool Call/Result；这些仍属于 Core `ToolScheduler`。
Action review、grant 与最终 execution decision 见 [`auto-review.md`](auto-review.md)。

## 2. Crate 边界

### 2.0 `zeta-install-context`

`zeta-install-context` 描述当前 executable 所在的 package layout，并提供 `zeta-path/` executable
candidates 与 `zeta-resources/` file candidates。它不选择 sandbox policy，不验证 helper
capability/digest，也不启动或复制资源。

当前 local `rg`、Linux Bubblewrap 与 Windows AppContainer composition 已消费该 contract。
canonical package 写入 `zeta-package.json` 与 `zeta-path/rg`；Linux 另带
`zeta-resources/bwrap`，Windows 另带 command runner 和 sandbox setup helper。平台 backend
在 host composition 时完成 candidate validation、capability/protocol probe 和 canonical
identity freeze。

### 2.1 `zeta-bwrap`

`zeta-bwrap` 同时提供 Linux Bubblewrap typed argv builder 与很薄的 upstream C entrypoint
wrapper：

- 接受显式 mount access、namespace、工作目录和 inner command；
- 始终使用 program/arguments，不经过 shell；
- 生成可检查的 `BwrapCommand`，不自行启动 Tool；
- package build 从 checksum-locked upstream source 编译 `bwrap_main`，不启用 setuid support；
- 不拥有 Zeta `SandboxPolicy`、Workspace grant、approval 或 fallback 决策。

调用方不能注入任意拼接后的 bwrap 参数。新增 Bubblewrap 能力应先成为 typed operation，再由
Linux policy adapter 决定是否使用。

### 2.2 `zeta-sandboxing`

`zeta-sandboxing` 是共享 contract 与 backend manager：

- `SandboxPolicy`、`FileSystemAccess`、`NetworkAccess`；
- `SandboxCommand` 与 `PreparedCommand`；
- `SandboxBackend` contract；
- `SandboxManager` 的 Workspace path validation 与 backend dispatch；
- 当前 macOS Seatbelt command transform；
- 现有 `WorkspaceRoot` containment。

macOS 实现暂时保留在本 crate，因为 Seatbelt transform 很薄，且平台选择与共享 policy 紧密。
当 macOS 原生实现需要独立 FFI、helper binary、较重依赖，或接近 500 LoC 时，再提取为
`zeta-macos-sandbox`；提取不能改变共享 policy。

### 2.3 `zeta-linux-sandbox`

`zeta-linux-sandbox` 把共享 policy 翻译为 `zeta-bwrap` operations：

- 非 FullAccess filesystem 默认以只读 root 开始；
- WorkspaceWrite 通过更具体的读写 mount 重开 Workspace；
- denied network 使用独立 network namespace；
- 添加 user/PID namespace、fresh `/proc`、`/dev`、session 与 parent-death containment；
- Bubblewrap 不可用或不支持所需能力时必须返回错误。

该 crate 当前拥有 system/bundled bwrap discovery、所需 CLI capability probe 与 canonical
identity freeze。后续仍拥有 version diagnostics、WSL 检查、seccomp 与 managed-network bridge；
这些细节不能进入共享 policy。

### 2.4 `zeta-windows-sandbox`

物理目录保留上游习惯名 `windows-sandbox-rs/`，Cargo package/API 名为
`zeta-windows-sandbox` / `zeta_windows_sandbox`。它拥有：

- shared policy 到 Windows filesystem/network authority 的解析；
- AppContainer profile/capability、ACL、child-process policy 与 Job Object enforcement；
- Windows helper/launcher 的生命周期和平台 diagnostics。

当前 `zeta-command-runner.exe` 先调用 `zeta-windows-sandbox-setup.exe` 创建或复用按
canonical Workspace + ro/rw mode 隔离的 AppContainer profile，为 Workspace 和冻结的 inner
executable 安装 ACL，再以零 capability AppContainer token 启动进程。零 network capability
承担断网，profile SID + ACL 承担文件访问；
child-process restriction 与单进程 Job Object 补充进程树控制。当前只支持
ReadOnly/WorkspaceWrite + NetworkDenied；其他受限组合 fail closed。

这不是 Codex dedicated local user + WFP firewall 实现的复制。Zeta v1 选择 Windows 原生
AppContainer 边界，并明确记录 ACL 是持久 filesystem metadata。helper 已接入 package、
discovery、App Server 与 MSVC target 交叉检查；真实 Windows AppContainer/ACL/network
integration tests 尚未完成，因此暂不标记为 production-enforced。
Windows 测试人员应按
[`windows-sandbox-acceptance-runbook.md`](windows-sandbox-acceptance-runbook.md)
回填实际结果、exit code、transcript 和 ACL 证据，再与固定 golden expectations 比对。

## 3. 依赖方向

允许：

```text
zeta-linux-sandbox   → zeta-bwrap + zeta-sandboxing
zeta-windows-sandbox → zeta-install-context + zeta-sandboxing
host composition     → zeta-install-context + platform sandbox + tool runtime
zeta-policy          → zeta-sandboxing
zeta-auto-review     → zeta-policy + zeta-sandboxing
host executor        → zeta-sandboxing + 当前平台 backend
```

禁止：

```text
zeta-bwrap → zeta-sandboxing / protocol / core
platform sandbox → zeta-core / ThreadStore / approval UI
zeta-sandboxing → shell-command / file-system / apply-patch / app-server / provider
zeta-sandboxing → zeta-policy / zeta-auto-review
zeta-install-context → zeta-sandboxing / platform sandbox / shell-command
```

平台 backend 通过 `SandboxBackend` 注入。共享 manager 不依赖所有平台实现，因此不会形成
`sandboxing ↔ platform crate` 循环，也不会把 Windows native 依赖带入 Linux/macOS binary。

## 4. 安全不变量

- 非 `FullAccess + AllowedNetwork` 请求必须由平台 sandbox enforcement 承担；
- backend 缺失、版本过旧或 policy 无法完整表达时 fail closed；
- `WorkspaceRoot` path containment 不是 OS sandbox，不能作为 fallback；
- model/tool arguments 不能选择 backend、扩大 mount、授予网络或要求降级；
- command 与 bwrap 参数始终以结构化 argv 传递；
- symlink、non-existent write path、nested deny/readonly carveout 必须在进入真实执行前处理；
- capability probe 与实际 spawn 使用同一 resolved executable，避免检查/执行竞态；
- diagnostics 必须区分 unavailable、unsupported policy、setup failure 和 sandbox denial。

## 5. 实施顺序

1. typed policy、backend contract 与 command construction；
2. 将 process executor 改为消费 `PreparedCommand`；
3. Linux bwrap discovery/probe 与 bundled package；
4. Linux 真实 namespace integration tests 与 seccomp；
5. Windows AppContainer launcher、ACL/network enforcement 与 Windows CI；
6. macOS Seatbelt profile compatibility/integration tests；
7. managed network proxy、PTY 与 cancellation/kill-tree integration。

Linux bundled source/build/discovery 已完成；真实 Linux namespace integration 与 seccomp 仍是
当前限制。Windows helper/build/discovery/enforcement path 已完成，仍需真实 Windows
integration tests 后才能标记为 production-enforced。
