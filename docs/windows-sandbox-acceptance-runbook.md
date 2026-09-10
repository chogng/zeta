# Windows MXC 验收手册

本手册验证 `CommandExecutor → mxc-sandbox → Microsoft MXC SDK → ProcessContainer`。
2026-09-10 在 Windows 11 23H2 实机执行：已修复不存在保护路径导致的错误；实机仍受 `C:\` ACL 权限限制，尚未通过系统隔离验收。
实现契约见 [mxc-sandbox](../zeta-rs/mxc-sandbox/README.md)。

## 入口

不安装 Zeta command runner、配置服务、worker 或 Runtime MSI。
记录 Windows build、架构、SDK revision、执行策略和 SDK 诊断，不能仅按是否存在 PSEC 判断整个产品是否可用。

```powershell
just check zeta-mxc-sandbox --tests
just test zeta-mxc-sandbox --test windows
just test zeta-mxc-sandbox --test windows -- --ignored --test-threads=1
```

非忽略测试验证当前严格受管网络请求被拒绝，且用户命令没有启动。
忽略测试要求真实文件隔离与进程树能力，验证目录范围、退出码、取消和超时。
测试显式允许 SDK 为策略中的路径配置宿主 ACL；能力或权限不满足时记录失败，不把未执行当作通过。

## 必须验证的行为

| 场景 | 预期 |
| --- | --- |
| 多根文件范围 | 工作目录可写，参考目录只读，其他 Agent 目录不可读 |
| 元数据 | `.git` 等保护路径不可写，路径别名不能扩大权限 |
| 退出码 | 用户进程 `125` 保留为普通退出码，不伪装成私有运行器启动失败 |
| 网络禁止 | 禁止直连与宿主回环，不依赖代理环境变量实现隔离 |
| 严格受管网络 | 当前部署明确拒绝，不能放开入站或一般回环来使其运行 |
| 后端选择 | SDK 仅使用能够实施本次请求且符合宿主 ACL 要求的实现 |
| 取消、超时、关闭 | 主进程及后代结束，无延迟文件副作用 |
| 宿主 ACL | 对比执行前、正常关闭后和异常退出后的 ACL；分别记录清理与恢复结果 |
| 包 | 不再要求自建沙箱辅助程序，保留 MXC 许可证 |

还需手工核对链接与准备后替换路径、并发 ACL 改动、IPv6、宿主崩溃后的资源恢复。

## WSL 验收边界

Windows、WSL 2 中的 Linux 进程和 MXC 的 WSL Container（WSLC）是不同的执行路径，验收结果不能互相替代。

| 场景 | 验收要求 | 当前范围 |
| --- | --- | --- |
| Windows 版 Zeta 执行 Windows 命令 | 执行本手册的 ProcessContainer 验收 | 必须完成，本次实机未通过 |
| Windows 受限命令调用 `wsl.exe` | 检查能否跨入 WSL 后越权访问文件、直连网络或留下存活进程 | 纳入 Windows 绕过检查；需要可正常执行命令的 WSL 环境 |
| WSL 2 内运行 Linux 版 Zeta | 在 WSL 2 内执行 Linux 沙箱验收，并检查跨系统边界 | 若将此用法列入支持范围，发布前必须单独通过 |
| Windows 通过 MXC WSLC 启动 Linux 容器 | 验证 WSLC 的文件、网络、输入输出及完整容器生命周期 | 当前未启用，不属于已接入功能的验收 |
| WSL 1 内运行 Linux 版 Zeta | 独立验证其系统能力，不能沿用 WSL 2 结果 | 本轮不作支持或验收通过声明 |

当前 Zeta 未开启 `mxc-sdk` 的 `wslc` feature，也未选择 `Containment::Wslc`。
固定上游版本将 WSLC 列为需要显式启用的实验能力，见 [WSLC SDK 说明](https://github.com/microsoft/mxc/blob/6cd3d58f05d3447e67109cfb75e042803b843ca4/docs/wsl/wsl-container-getting-started.md#rust-sdk)。

WSL 2 的 Linux 验收复用 [Linux 测试入口](../zeta-rs/mxc-sandbox/README.md#验证)，另外必须覆盖：

- Linux 文件系统工作目录与 `/mnt/c` 工作目录分别验证目录授权、只读元数据、隐藏目录和路径别名。
- 检查通过 Windows 可执行文件及 WSL 互操作入口，能否越过文件和网络限制；取消后检查两侧进程和延迟写入。
- 对承诺支持的 NAT、mirrored 网络模式分别验证代理、Windows 宿主地址、回环、DNS、IPv4 和 IPv6。
- 缺少用户命名空间、Bubblewrap 或网络隔离依赖时明确拒绝启动，不把未执行计为通过。

WSL 2 使用 Linux 内核，但提供跨系统文件与命令互操作；mirrored 模式还改变宿主回环的可达性。
因此普通 Linux CI 通过不足以证明上述边界通过，见 [WSL 版本区别](https://learn.microsoft.com/en-us/windows/wsl/compare-versions)、[文件与命令互操作](https://learn.microsoft.com/en-us/windows/wsl/filesystems)、[网络模式](https://learn.microsoft.com/en-us/windows/wsl/networking)。

## 严格代理的缺口

当前 Zeta 要求只连自己的代理并保持其他入站和回环关闭。
固定版本的 ProcessContainer 代理模式需要相应代理身份及不同的私网入站配置，普通宿主代理不满足这一契约。
SDK 返回不支持；不能用成功启动、HTTP_PROXY 存在或更宽的网络配置作为验收通过。
要开放这项能力，必须先补齐实际部署与 SDK 能力，并重新进行网络绕过和并发执行验收。

## 证据与发布状态

保存完整命令、退出码、标准流、SDK 诊断、文件和 ACL 差异、存活进程检查。
Windows 实机及生产隔离资格尚未取得；下列部分结果不能代替完整验收。
Microsoft 对固定预览版的限制见 [上游说明](https://github.com/microsoft/mxc/tree/6cd3d58f05d3447e67109cfb75e042803b843ca4)。

### 2026-09-10 首次实机记录

- 源码：`11496080773c6598b99a7539148c50b59b20d676`，MXC revision 同本文固定版本。
- 系统：Windows 11 专业版 23H2，build `22631.6199`，`x86_64-pc-windows-msvc`；Rust `1.98.0`。
- 执行环境：未提升权限，PowerShell `LocalMachine=RemoteSigned`，其他执行策略范围均为 `Undefined`。
- 本机完整构建和测试输出：`.build/acceptance/mxc-windows-20260910/check.log`、`windows.log`；这些是本机证据文件，不随 Git 提交。

| 命令 | 退出码 | 结果 |
| --- | --- | --- |
| `just check zeta-mxc-sandbox --tests` | 0 | 通过，未报告编译 warning |
| `just test zeta-mxc-sandbox --test windows -- --include-ignored --test-threads=1` | 1 | 1 项通过、2 项失败、0 项忽略；测试构建完成，未报告编译 warning |
| `wsl --status` | 50 | 无可用状态输出 |
| `wsl --list --verbose` | 1 | 返回帮助文本，未取得可执行的发行版信息；WSL 测试未执行 |

通过的是严格受管网络拒绝测试：拒绝请求，且命令写入标记和代理回调均未出现。
这只证明拒绝行为，不表示 Windows 已支持严格受管网络。

两个失败均发生在 SDK 启动阶段：SDK 报告 BaseContainer 不可用，对文件策略中的不存在路径检查 `WRITE_DAC` 时返回 `os error 2`。
多根目录测试失败于 `work/.agents`；进程树测试失败于 `.git`，尚未进入超时和取消断言。
当时适配器把全部保护目录名加入只读策略，SDK 对这些路径使用 `OPEN_EXISTING` 检查访问权；当次错误是路径不存在，不能仅根据外层错误文案判断为用户缺少 ACL 权限。

文件隔离、退出码、进程树终止、ACL 正常与异常恢复、网络绕过和 WSL 跨系统边界均仍待验收。
### 2026-09-10 修复与复测

适配器按元数据契约检查路径是否存在：已存在的文件、目录及链接仍进入只读策略，确认不存在的路径不提交给 ACL 实现，也不创建空目录。
检查遇到其他错误时拒绝请求；工作目录授权、隐藏存储和网络要求继续保留。
新增回归覆盖空工作目录，以及同时存在元数据文件、目录和缺失路径的多根目录策略。

`platform-checks.yml` 已改为 `--include-ignored`，Windows CI 同时执行严格网络拒绝和实机用例。
进程树测试名称及忽略说明改为描述 MXC 能力要求，移除已退场的自建运行器前置条件。

复测时，两个实机用例均已越过缺失元数据检查，但 SDK 报告 BaseContainer 不可用，且当前账户不能对 `C:\` 执行 `WRITE_DAC`，因此仍在启动阶段失败。
这是宿主只读基线所需权限未满足，不能删去磁盘根目录要求、改为普通进程或将失败改成跳过。
继续验收需要具备相应系统隔离能力、且能完整满足文件策略的执行环境；本次没有更改宿主 ACL 或系统功能配置。

| 修复后命令 | 退出码 | 结果 |
| --- | --- | --- |
| `just test zeta-mxc-sandbox --lib` | 0 | 4 项通过，含 2 项新增文件策略回归 |
| `just check zeta-mxc-sandbox --tests` | 0 | 通过 |
| `python -B scripts/cargo.py build -p zeta-mxc-sandbox` | 0 | 正常构建通过 |
| `just test zeta-mxc-sandbox --test windows -- --include-ignored --test-threads=1` | 1 | 1 项通过、2 项失败、0 项忽略；失败原因均为上述 `C:\` 权限限制 |

本轮编译未报告 warning。对应本机日志位于同一证据目录的 `fix-lib.log`、`fix-check.log`、`fix-build.log`、`fix-windows-final.log`。
