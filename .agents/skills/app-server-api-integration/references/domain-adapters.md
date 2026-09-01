# 领域适配

决定文件、终端、搜索等前端契约、channel 或 adapter 放在哪里时，完整读取本文件。

## 按职责判断 owner

一个领域可以跨越多个 VS Code 层；要保证的是每项职责只有一个 owner，而不是一个领域只能选择一个目录。

| 职责 | 通常位置 |
| --- | --- |
| 跨调用方共享的领域接口、值和事件 | `platform/<domain>/common` |
| renderer 侧领域 channel client | VS Code 对应的 `platform/<domain>/electron-browser`、`workbench/services/<domain>` 或功能目录 |
| Electron Main 侧领域 `IServerChannel` adapter | VS Code 对应的 `platform/<domain>/electron-main` 或明确拥有后端接入的功能目录 |
| workbench 生命周期、注册与产品级组合 | `workbench/services/<domain>` |
| 功能本身的 UI、命令与接入协调 | `workbench/contrib/<feature>` |
| Rust 领域行为、校验和后端状态 | 实际领域 crate，由 `zeta-rs/app-server` 分派 |

先在 `../vscode` 证明现有 owner 和调用方向。不要为目录整齐而复制 common interface、adapter、状态或注册。

## adapter 边界

领域 adapter 只负责：

- 把 VS Code 领域值转换成 `zeta-ts/generated/app-server/types.ts` 中的 DTO。
- 保存 request、watch、stream、terminal 等关联标识。
- 把 Rust 领域错误恢复成调用方已经处理的 VS Code 错误语义。
- 把通知和终止信号转换为现有事件，并在对象释放时结束对应资源。

领域 adapter 不负责权限策略、持久化、排序规则、业务校验、后端缓存或系统访问。common 领域契约通常也不导入生成 DTO；如果产品代码必须理解线上字段，说明 adapter 边界尚未完成。

## 文件系统示例

VS Code 的文件能力本身就跨层：platform 定义文件服务和 provider 契约，Electron Main 提供后端 channel，workbench 组合具体 provider。Rust 替换的是系统访问和后端状态，不是 `IFileSystemProvider` 等前端语义。

调查入口：

- `../vscode/src/vs/platform/files/common/diskFileSystemProviderClient.ts`
- `../vscode/src/vs/platform/files/electron-main/diskFileSystemProviderServer.ts`
- `../vscode/src/vs/workbench/services/files/electron-browser/diskFileSystemProvider.ts`

Rust 协议必须完整表达调用方实际使用的 stat、read、write、rename、delete、目录操作、watch、流和错误。若当前 Codex API 没有这些能力，就在 Rust 增加，而不是缩减 VS Code provider。

## 终端示例

终端同样可以合理跨越 `platform/terminal` 与 `workbench/contrib/terminal`：前者拥有共享终端契约，后者拥有 workbench 终端接入和 UI 生命周期。不能因为后端改为 Rust 就把终端全部移动到 `platform/app-server`。

调查入口：

- `../vscode/src/vs/platform/terminal/common/terminal.ts`
- `../vscode/src/vs/workbench/contrib/terminal/common/remote/remoteTerminalChannel.ts`
- `../vscode/src/vs/workbench/contrib/terminal/browser/remoteTerminalBackend.ts`

Rust 应拥有进程创建、输入、resize、持续输出、退出状态和后端资源释放；TypeScript adapter 将这些能力还原成 `ITerminalBackend` 等已有前端契约。

## 搜索及其他长任务

搜索、索引、日志或持续诊断通常不是单个 Promise。先确认现有 provider/service 的结果批次、进度、取消和完成事件，再选择 request、stream 或带标识的资源订阅。不要为了统一接口把长任务塞进巨大 `IAppServerApi`。

## 文件命名与新增路径

- 优先沿用 VS Code 对应职责的公开接口、目录和命名。
- Zeta 特有的线上 DTO 和连接实现使用 `app-server` 目录，但领域契约仍留在领域 owner。
- 新建 VS Code 中没有对应职责的 TypeScript 公开路径前，先说明为什么现有 owner 无法承接并取得用户确认。
- 不为每个领域创建相同模板文件；只有真实调用方、进程边界或生命周期需要时才拆文件。
