# 验收记录

基线：`702f2ea63`；开始时工作区干净。本候选包含 OpenAI 子页、Config 请求路由、相关行为测试、PTY 场景入口更新与文档。环境为 Windows PowerShell。

| 要求 | 证据 | 结果 |
| --- | --- | --- |
| AC-1 | editor 测试；60/100 列完整 App 字符输出测试 | 通过 |
| AC-2 | 输入隐藏、取消、应用层保存返回、失败保留输入测试 | 通过 |
| AC-3 | 独立 provider 身份、revision 刷新、配置请求内容与拒绝响应测试 | 通过 |
| AC-4 | 现有订阅测试；应用层登录期间返回后异步结果不重开页面、取消登录测试 | 通过 |
| AC-5 | 编辑器键盘和点击入口测试；输入期间点击不能激活背后列表 | 控件层通过；真实终端鼠标未实测 |

执行结果：

- `just check zeta-tui`：通过，有既有未使用代码警告。
- `just test zeta-tui --lib config::`：22 通过。
- `just test zeta-tui --lib openai`：3 通过，其中 2 项与上一组重叠。
- `just test zeta-tui --lib provider_api_key`：3 通过，其中 1 项与配置组重叠。
- `just test zeta-tui --lib chatgpt_subscription_keeps`：1 通过。
- 共 26 项不同测试通过。字符输出检查覆盖菜单标签和 Key 不明文显示，未新增截图基线。

首次编译发现 ConfigureProvider 未加入请求调度分组，补入 Config 分组后检查通过。新增 Unix PTY 场景入口已跟随新层级修改；因现有测试使用 `cfg(unix)`，本 Windows 环境没有运行真实 PTY 场景。未使用真实凭据或调用收费模型。当前是自查，没有独立审查。

本轮可供用户运行 `just zeta`，通过 `/config → Providers → OpenAI` 体验。任意多个命名连接与远程模型目录发现尚未实现，不能把本轮面板交付当作完整连接管理已经完成。
