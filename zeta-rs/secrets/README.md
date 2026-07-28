# `zeta-secrets`

> 本 README 解释 opaque secret value、store port 与 backend obligation。跨系统 credential
> ownership 和 backend policy 见 [`docs/secrets.md`](../../docs/secrets.md)。

`zeta-secrets` 只保存 opaque bytes。OAuth、token refresh、account metadata、credential scope、
request signing 和 Provider header materialization 均属于消费它的 domain runtime。

## Public contract

| Symbol | 职责 | 安全语义 |
| --- | --- | --- |
| `SecretKey` | 非 secret 的 stable lookup identity | 非空、最多 512 bytes、禁止 control character |
| `SecretValue` | opaque secret bytes | 不实现 Clone/Display/Serialize；Debug 固定 redacted；Drop zeroize |
| `SecretStore` | `load / store / delete` port | namespace isolation、sanitized error、完整三操作 |
| `DeleteSecretOutcome` | exact delete result | 区分 `Deleted` 与 `NotFound` |
| `MemorySecretStore` | process-local ephemeral backend | replacement、delete、drop 时 zeroize stored bytes |
| `UnavailableSecretStore` | explicit fail-closed backend | 所有操作返回 `BackendUnavailable` |
| `SecretStoreError` | sanitized error | message 不能包含 secret/header/raw backend response |
| `SecretStoreErrorKind` | stable caller classification | unavailable、access denied、backend failure |

`SecretKey` 的内容会出现在 `Debug`，因此 key schema 不能包含 token、email 或其他敏感 identifier。
Key schema 由调用 domain 拥有，本 crate 只验证基本安全 shape。

## 内部接口与调用路径

| Symbol | 可见性 | 当前职责 |
| --- | --- | --- |
| `SecretValue::expose` | public explicit borrow | 唯一取得 plaintext bytes 的路径；borrow 应尽量短 |
| `MemorySecretStore::values` | private mutex map | 保存 owned byte copies，不保存 `SecretValue` clone |
| `lock_error` | private function | poisoned mutex → sanitized `BackendFailure` |
| `unavailable` | private function | 三个 unavailable operations 共用稳定错误 |
| `Drop for SecretValue` | impl | zeroize caller-owned secret buffer |
| `Drop for MemorySecretStore` | impl | zeroize map 中所有 surviving values |

```text
store(key, SecretValue)
├─ SecretValue::expose (short borrow)
├─ copy bytes into backend
├─ replace old bytes
└─ zeroize replaced buffer

load(key)
├─ backend copies opaque bytes
└─ SecretValue::new → caller-owned zeroizing value

delete/drop
└─ zeroize backend-owned bytes before release
```

方向偏差：

- `SecretValue` 增加 `Clone`、`Display` 或 serde：secret 传播面失控；
- backend error 包含 command/header/raw response：sanitization boundary 被破坏；
- `MemorySecretStore` 被当作 production persistence：进程退出即丢失；
- unavailable host 返回空值而非 error：credential absence 与 backend failure 混淆；
- API/network layer 直接读 `SecretStore`：credential lifecycle ownership 下沉错误。

## Backend implementation checklist

Production backend 必须实现完整 load/store/delete、隔离 Zeta namespace、定义 overwrite atomicity、
sanitized errors 和 negative logging tests。只有 load 没有 delete 的 backend 不满足 trait 语义。

内存 zeroize 降低残留风险，但不保证编译器、allocator、swap、core dump 或 backend copy 中不存在
副本；系统级保护仍由 OS facility、process hardening 和 caller lifecycle 负责。

## 测试与修改路径

```text
cargo test -p zeta-secrets
bazel test //zeta-rs/secrets:secrets-unit-tests
```

当前测试覆盖 round-trip/replace/delete、Debug redaction、key validation 和 unavailable error。
新增 backend 时必须补 error/log/debug negative tests，并证明 replacement/delete/drop 的 secret
buffer 处理。

## 当前限制与演进

当前只有 memory 与 unavailable backend。OS keyring、encrypted file、migration、rotation metadata
均尚未实现。它们应作为 sibling private module 接入同一 `SecretStore` contract；不要为 backend
能力扩大 `SecretValue` 的复制、序列化或日志接口。
