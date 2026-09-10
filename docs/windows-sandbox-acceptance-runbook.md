# Windows MXC 验收手册

本手册验证 `CommandExecutor → mxc-sandbox → Microsoft MXC SDK → ProcessContainer`。
真实 Windows 结果尚待回填；交叉编译不计为系统隔离验收。
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

## 严格代理的缺口

当前 Zeta 要求只连自己的代理并保持其他入站和回环关闭。
固定版本的 ProcessContainer 代理模式需要相应代理身份及不同的私网入站配置，普通宿主代理不满足这一契约。
SDK 返回不支持；不能用成功启动、HTTP_PROXY 存在或更宽的网络配置作为验收通过。
要开放这项能力，必须先补齐实际部署与 SDK 能力，并重新进行网络绕过和并发执行验收。

## 证据与发布状态

保存完整命令、退出码、标准流、SDK 诊断、文件和 ACL 差异、存活进程检查。
当前只完成代码接入和跨平台检查；Windows 实机及生产隔离资格尚未取得。
Microsoft 对固定预览版的限制见 [上游说明](https://github.com/microsoft/mxc/tree/6cd3d58f05d3447e67109cfb75e042803b843ca4)。
