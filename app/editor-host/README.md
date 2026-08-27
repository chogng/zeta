# `zeta-editor-host`

1. 管文件 Tab、活动文档、视口、脏状态、外部修改冲突和关闭/保存周期。
2. 管编辑器搜索、自动滚动、诊断、补全和悬浮信息等保留视图状态。
3. 产品宿主只执行文件与语言服务请求并转发平台输入；文本编辑能力继续委托给 `zeta-editor`。

验证：`cargo test -p zeta-editor-host --lib`。
