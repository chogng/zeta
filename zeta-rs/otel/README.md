# zeta-otel

> 当前状态：只保留 OTel 职责边界和隔离的 in-memory mock；尚未接入产品运行链路。

`zeta-otel` 预留与 `codex-otel` 同类的集成职责。未来由它统一承载 OTel provider、exporter、logs、traces、metrics、trace context 和生命周期管理。

## 当前隔离方式

| 文件 | 作用 | 默认构建是否包含 |
| --- | --- | --- |
| `src/lib.rs` | crate 边界，仅声明 feature-gated mock module | 是 |
| `src/mock.rs` | in-memory metrics/spans、事件适配和 monitor snapshot | 否 |
| `src/otel_tests.rs` | mock 测试 | 否 |

`mock` 是非默认 feature。`model-provider` 和其他业务 crate 不依赖它，也没有因此增加运行时行为。

```bash
# 验证默认边界
cargo check -p zeta-otel

# 显式验证 mock
cargo test -p zeta-otel --features mock
```

内部可视化 monitor 以后可以显式启用 `mock`，读取内存 snapshot。正式 OTel exporter 应作为独立模块实现，不在 `mock.rs` 上继续叠加。

## 数据约束

- 默认只记录安全的结构化字段，例如 provider、model、method、status 和 duration。
- 不记录 prompt、response、token、authorization header 或其他用户内容。
- mock 不启动网络请求，也不向外部 collector 导出数据。
