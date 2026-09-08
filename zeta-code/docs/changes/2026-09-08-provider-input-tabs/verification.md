# 验收

2026-09-08，要求版本见 [spec.md](spec.md)。当前候选包含接手时的 8 个后端未提交文件；不能只用 HEAD 标识。测试和构建尚在执行，结果随后追加。审查方式为自查。


## 首轮结果

- `just check zeta-tui`：通过（初始候选，后续补充了错误分类、提示和测试，最终构建另行记录）。
- `just test zeta-tui --lib config::`：第一次因 Status 测试尚未适配可空的活动页签而编译失败；修正后 40 项行为测试通过，1 项字符快照失败。
- 80×24、36×14 快照分别报告字段标记变化。逐行审查后接受两份快照；没有秘密值或其他布局变化。环境没有 cargo-insta，用精确路径替换已审查的 `.snap.new`。
- `just generate-protocol`：成功生成 Rust 协议 fixtures 和 TypeScript 客户端。
- `just test zeta-tui --lib`：706 项通过、2 项失败、1 项忽略。两项失败是 App 测试仍使用进入表单即编辑和保存后自动移到 Fetch 的旧步骤；已更新按键路径，待重跑。
- `just test zeta-model-provider --lib catalog::`：6 项通过，包括 OpenAI/Ollama 的 HTTP 分类、客户端错误分类、空目录、凭据隔离和范围失效。

环境：Windows，真实 PTY 场景文件是 `cfg(unix)`，本轮没有执行该套场景；不以零测试通过代替终端实测。无真实服务账号调用。

- `just test zeta-app-server --lib provider`：17 项通过。包含实际 ConfigBackedModelService 的成功→空目录→401→重试，以及 RPC 的 models/empty/failed 三类结果。测试编译报告既有 `local_tools.rs` 测试辅助函数未使用警告。
- 生成文件逐字节核对、变更 Markdown 的本地链接检查、变更 Rust 文件 `rustfmt --check`（不递归子模块）和 `git diff --check`：通过。

- 提取基座前，配置 41 项、两项 App 回归重跑均通过；协议 38 项通过，最终普通构建通过。

## v2 输入控件候选

用户补充 AC-7 后提取通用 TextField。前述后端和协议验证仍适用；TUI 实现已变化，因此重新运行 TUI 测试与普通构建，不沿用 v1 的 TUI 完成结论。


## v2 最终验收 · 2026-09-08

输入：AC-1～AC-6 加用户补充的 AC-7。候选基线 `7f935080d0c39b1cdfebac25d93dceb7235bafae`，源码清单见 [source.sha256](source.sha256)，清单 SHA-256 `8d4053455cd0157571a85373a0b3cf883b61d895383ba9b9bbee3fc557c061b9`；清单包含当前修改的源码、两份快照和协议生成产物，排除验收文档。

| 要求 | 实现与证据 | 结果 |
| --- | --- | --- |
| AC-1～AC-3 | 名称、URL、Key 复用 TextField；配置测试 41 项通过，App 的编辑→保存→手动选择 Fetch 与 Esc 取消场景通过 | 通过 |
| AC-4 | Model Provider 6 项、App Server provider 17 项、协议 38 项通过；覆盖成功、空列表、分类失败、重试、旧请求和只读刷新 | 通过 |
| AC-5 | TabList 的 9 项测试在 TUI 整包测试中通过，包括禁用跳过、禁止直接选择、全部禁用、恢复与样式 | 通过 |
| AC-6 | 80×24、36×14 快照及 Key 掩码断言通过；没有待接受快照 | 通过 |
| AC-7 | TextField 5 项测试通过；页面不再持有文字字段的编辑 bool、逐字段取消逻辑或已确认 Key 副本 | 通过 |

- `just test zeta-tui --lib`：712 项通过、1 项快照失败、1 项真实终端测试忽略。唯一失败是控件初始化保留了协议提示的三个空格；修正提示后 `just test zeta-tui --lib config::` 41 项全部通过，两份快照均通过。遵守定向重跑规则，未再次运行无关测试。
- `just check zeta-tui`：最终候选通过，没有编译警告。
- 前述后端 6 + 17 项、协议 38 项验证期间未修改后端逻辑，证据仍适用。
- 真实 PTY/真实账号未执行，范围与原因见首轮记录；本次完成的是输入控件、Provider 交互、模型发现接口和禁用页签的实现及自动化验收，不宣称真实服务或全部终端实测。
- 自查：确认控件与业务保存职责、异步结果身份、输入掩码、焦点边界、生成物一致性和最终差异。没有独立审查。


最终工作区范围核对：验收收尾时另有 `app/event_loop.rs`、`app/state.rs`、`thread/transcript/history.rs` 的并行改动出现。它们不属于本轮实现，也不计入本轮源码清单。整份工作区格式检查遇到其中两文件的格式差异；本轮文件的独立格式检查另行通过。前述测试仅绑定所记录的本轮候选，不宣称覆盖后来出现的其他改动。


## 后续真实终端验收

2026-09-08 在 Windows ConPTY 完成 Provider 的编辑、取消、保存、模型发现及退出场景；具体代码候选、命令、结果和共享管道关闭修复见 [PTY 验收](../2026-09-08-pty-windows/verification.md)。此前“未实测”保留为当时记录，真实账号仍未调用。
