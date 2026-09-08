# 验收记录

本记录覆盖 tab 表单及 Enter 确认后自动进入下一字段的改造，取代旧列表面板的验收。需求版本见[规格](spec.md)，代码基线为 `dc9760352`，环境为 Windows PowerShell，验证日期为 2026-09-07～08。本轮代码、快照与需求文件的候选指纹见 [source.sha256](source.sha256)。工作区同时存在目录添加相关改动，未撤销这些改动；共享文件的指纹包含验证时的完整内容。

| 要求 | 实现 | 实际证据与结果 |
| --- | --- | --- |
| AC-1 | 已实现 | 创建后保留最后的新建 tab、返回 Providers 后重开仍能编辑新连接；编辑器与完整 App 字符输出测试通过 |
| AC-2 | 已实现 | 固定官方地址、隐藏 Key、协议选择及 80×24 / 36×14 字符快照通过；两种协议的实际请求路径由录制传输测试验证 |
| AC-3 | 已实现 | Enter 后等待保存结果；成功才自动进入下一字段，失败保留输入；最后一项只聚焦按钮；状态与 App 命令测试通过 |
| AC-4 | 已实现 | 草稿跨 tab 保留、显式创建、改名保持身份、新建空 Key 可用；独立凭据与持久化测试通过 |
| AC-5 | 已实现 | App Server 测试确认普通模型列表读取不发起自定义目录请求，主动获取后结果进入模型选择。空目录、非法响应、HTTP 错误、重试及地址/协议/Key 变更后的目录隔离测试通过 |
| AC-6 | 已实现 | 现有账户请求及 App 切页测试通过，包括登录未完成时离开、返回后取消，以及旧事件不重开面板；未使用真实账号 |
| AC-7 | 已实现 | 键盘连续填写、返回、只读字段跳过、宽窄字符布局测试通过；固定面板保持终端鼠标模式。Unix PTY 场景入口已更新，Windows 环境未运行这些场景 |
| AC-8 | 已实现 | 真实临时配置存储关闭重开、过期 revision 拒绝、命名连接配置与凭据 RPC 往返、异步回复不抢回当前 tab 的测试通过 |
| INV-1～3 | 已实现 | 配置与 Key 分别通过配置接口和 secret store 写入；测试确认请求和诊断不混入另一连接的 Key，协议不自动切换，模型获取失败保留已保存配置 |

执行结果：

| 命令 | 结果 |
| --- | --- |
| `just test zeta-tui --lib config::` | 31 通过 |
| `just test zeta-tui --lib openai` | 11 通过，其中 10 项与配置组重叠；最终 UI 提示调整后再次通过 |
| `just test zeta-tui --lib config_provider_api_key` | 1 通过 |
| `just test zeta-tui --lib escape_cancels_key` | 1 通过 |
| `just test zeta-tui --lib chatgpt_subscription_keeps` | 1 通过 |
| `just test zeta-tui --lib fixed_command_panels_ignore_mouse` | 1 通过 |
| `just check zeta-tui` | 最终非测试构建检查通过；存在既有未使用代码警告 |
| `just test zeta-app-server --lib custom_provider` | 2 通过 |
| `just test zeta-models-manager` | 11 通过 |
| `just test zeta-model-provider-config` | 20 通过 |
| `just test zeta-model-provider` | 首轮 56 通过、1 失败、1 项既有忽略；修正下述测试样本后失败项通过 |
| `just test zeta-model-provider runtime_accepts_structured_tool_requests` | 1 通过；为旧严格工具样本补齐 properties、required 与 additionalProperties |
| `just test zeta-model-provider openai_catalog_handles` | 新增的空目录和 HTTP 错误测试，1 通过 |
| `just test zeta-config` | 首轮 50 通过、1 失败；修正下述 Windows 测试样本后失败项通过 |
| `just test zeta-config unversioned_config_is_migrated_and_rewritten_once` | 1 通过；旧样本改用目录 owner 的规范路径，并通过 TOML 值序列化路径，未修改迁移实现 |
| `just generate-protocol` | JSON Schema、TypeScript 方法映射和运行时解码器生成通过 |
| `git diff --check` | 通过 |

按不同测试计算，共 178 项通过，另有 1 项既有忽略。两处旧测试样本修复均只重跑失败用例，没有放宽生产校验或测试断言。

新增两份字符快照，分别覆盖宽窗口完整表单与窄窗口当前字段可见。快照先产生 `.snap.new`，逐行检查后按精确路径接受，再运行断言通过；本机未安装 cargo-insta，因此接受步骤使用精确文件重命名。旧的完整进程 Key 输入画面基线随旧页面退场，详细布局由上述 App/组件快照覆盖，Unix 场景保留真实进程输入与保存检查。

当前为自查。未运行整个 Rust workspace，未调用真实收费模型，也未使用真实账号或 Key 验证系统凭据存储。真实终端键盘体验和真实账号登录仍需产品环境确认；上述通过结论限于实际运行的状态、协议、持久化、字符输出和构建检查。

## 2026-09-08 焦点职责修正候选

本候选基于 `7ee335295c` 加当前未提交差异，重新验证细化后的 AC-7。`TabList` 统一解释页签区的切页、Enter 和向下导航；表单与嵌入列表只报告边界，OpenAI 面板协调区域焦点和切页业务动作。嵌入的 ChatGPT 账户列表不再保留未绘制的内部页签。

| 验证 | 结果 |
| --- | --- |
| `just test zeta-tui --lib widgets::` | 60 通过；覆盖共享页签按键、列表边界和焦点区域规则 |
| `just test zeta-tui --lib openai` | 13 通过；覆盖首字段向上进入页签、页签内切换、账户正文返回页签和末操作向下停止 |
| `just test zeta-tui --lib config::` | 33 通过；覆盖配置页与 OpenAI 表单的组合交互 |
| `just test zeta-tui --lib every_directory_is_reachable_and_removing_the_last_clears_selection` | 1 通过；确认页签 Enter 仍直接进入列表，向下仍按视觉顺序进入搜索或列表 |
| `just check zeta-tui` | 通过；仅有既有未使用代码警告 |
| `git diff --check` | 通过 |

首次整包测试发现把页签 Enter 与向下合并为同一结果会让目录页 Enter 先进入搜索框；拆成“直接进入正文”和“聚焦下一块”后，目录回归测试通过。最终 `just test zeta-tui --lib` 为 692 通过、5 失败、1 忽略；5 项失败均由现有 `/memory` 命令已经进入目录而对应顺序、可见行、命中和字符快照尚未更新引起，不涉及本次焦点文件，因此整包不记为通过。测试生成的 `.snap.new` 已删除，本次没有接受或修改字符快照。

上述五项失败随后由[内存诊断配置开关](../2026-09-08-memory-diagnostics-config/README.md)删除 `/memory` 入口后恢复通过；原结果保留为本候选当时的验收事实。
