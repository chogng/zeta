# `zeta-file-access`

- `Dir` 用 `EnvId + canonical path` 标识目录并约束路径解析；路径、`cwd` 和 Workspace 都不授予权限。
- `Grant` 明确绑定 `GrantSubject`、目录范围、`Permissions`、来源和撤销生命周期；`Access` 与 `Snapshot` 管理同一主体的有效 Grant。
- `AuthorizationDecision = Result<Authorization, PermissionDenied>` 判断单次操作；允许值只供当前操作立即消费，撤销 Grant 后失效。长期语义见 [`docs/environment-access.md`](../../docs/environment-access.md)。
