# Windows sandbox

- 实现 Windows 专用账户、限制令牌、文件授权、WFP 网络限制与进程树回收。
- 隔离 Windows API 依赖，消费 `sandboxing` 的统一策略和 `install-context` 的安装身份。
- 安装和删除由独立命令执行，必须提供当前变更清单的摘要；普通执行不自动安装或修复。
- 每次执行独占账户、随机文件 SID、Job、桌面、代理路由和 ACL 恢复记录。
- 产品包提供独立 helper；运行时校验 helper 路径与 SHA-256，不搜索 PATH。

Windows 11 23H2 本机已通过 21 项单元测试和 9 项完整执行用例，包括 PowerShell、目录与元数据保护、IPv4 受管网络、IPv6 断网、双账户并发、取消和后代回收。

`FileSystemIsolation::WindowsAccount` 使用 Codex 的兼容令牌和有预算限制的可写路径审计，不承诺整个宿主只读。要求 `Strict` 时在启动前拒绝；未获授权的宿主审计修正不会自动执行。

`HostAclChanges::ScopedWithTraversal` 允许范围内的 ACL 调整，以及必要祖先目录的非继承属性查询和遍历。它不授予祖先目录枚举或文件读取权限。设备与命名对象目录授权已退出实现，普通执行不触发安装或提升权限。

契约、审计限制与实机证据见 [沙箱架构](../../docs/sandboxing.md) 和 [Windows 验收手册](../../docs/windows-sandbox-acceptance-runbook.md)。

```powershell
just test zeta-windows-sandbox --lib --locked
just check zeta-windows-sandbox --tests --locked
just rust-warnings zeta-windows-sandbox
python -B scripts/cargo.py build -p zeta-windows-sandbox --bin zeta-windows-sandbox --locked
bazel build //zeta-rs/windows-sandbox:zeta-windows-sandbox
```

需要真实账户的测试显式标为忽略，须在获准配置的独立验收环境执行。`scripts/test-windows-sandbox.ps1` 提供构建、安装、全部用例及 finally 清理入口；调用方须已获得安装与验收授权。

网络连接进程归属实现参考 Codex `da20788df913189878ebca7f4963d8a363ee6bf2`；对应许可与归属保存在 `LICENSE-APACHE` 和 `NOTICE`。文件 ACL 日志复用固定版本的 `wxc_common`，不引入 Codex 的协议或产品配置。
