# `zeta-file-system`

> 本 README 拥有目录内文件操作的实现契约；目录身份和授权由
> [`zeta-file-access`](../file-access/README.md) 负责。

- `FileSystem` 定义读取、写入、列举、重命名和删除文件的通用接口。
- `LocalFileSystem` 只在传入 `Dir` 的规范化边界内解析路径并执行宿主文件操作。
- 本 crate 不决定目录是否获权；调用方必须在构造或调用服务前检查对应 `Authorization`。
