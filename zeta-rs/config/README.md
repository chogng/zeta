# `zeta-config`

`zeta-config` 隔离配置文档、持久化事务和作用域合并，具体职责只有三项：

1. `ConfigStore` 按 `schemaVersion` 严格读取并原子迁移用户 `config.toml`；根级 `[gui]`、`[tui]` 作为前端自有键值表原样保存，SQLite 保存 revision、generation 和命令回执。
2. `DirConfigStore` 严格读取目录提供的 `.zeta/config.toml`；目录文档只能表达待处理的配置意图，不能给自身增加 capability。
3. `DirPermissionsConfig` 按 `DirId` 保存用户明确授予的 capability 集合；缺失条目表示没有持久授权，不使用 Trusted、Restricted 或目录级 Trust 状态。

配置合并、来源边界和运行时生效点见 [`docs/config.md`](../../docs/config.md)，环境与目录授权语义见
[`docs/environment-access.md`](../../docs/environment-access.md)。

```text
just test zeta-config
```
