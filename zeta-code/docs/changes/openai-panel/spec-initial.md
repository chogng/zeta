# 行为要求

- AC-1：Providers 中进入 OpenAI 后显示配置面板，区分 OpenAI API、自定义兼容服务与 ChatGPT 登录。Key 状态仅表示已保存，不能表示已验证连接。
- AC-2：Key 输入隐藏字符，保存复用 secret store；保存成功或取消返回 OpenAI，失败保留输入。
- AC-3：自定义地址通过 provider/configure 保存到 openai-compatible，保持其他配置字段，使用读取时的 revision；地址和 Key 分别保存，不能覆盖 openai 凭据。
- AC-4：ChatGPT 使用已有登录、取消和退出行为；Esc 返回 OpenAI，再按 Esc 返回 Providers。离开后异步登录结果不重开面板。
- AC-5：键盘与鼠标激活进入相同页面，输入期间不能点击到背后的列表。沿用通用列表与输入绘制。

模型选择仍由 /model 提供；本面板不宣称模型调用或网络认证已验证。
