# zeta-utils-path-uri

> 本 README 是 Rust `file:` URI 实现契约的 canonical owner。Desktop 通用资源 URI identity
> 由 [`zeta-ts/src/zeta/base/common/uri.ts`](../../../zeta-ts/src/zeta/base/common/uri.ts) 与
> [`resources.ts`](../../../zeta-ts/src/zeta/base/common/resources.ts) 拥有；跨层 ownership 与
> 当前状态见 [`docs/zeta-desktop-architecture.md`](../../../docs/zeta-desktop-architecture.md)；
> 本机 filesystem normalization 由 [`zeta-utils-path`](../path-utils/README.md) 拥有。

`zeta-utils-path-uri` 提供可序列化、跨 host 检查的绝对 `file:` URI。它用于需要在进程、RPC 或
remote execution boundary 上传递文件位置的 contract，不替代当前 App Server filesystem API
的 workspace-relative `PathBuf`，也不授予 URI 指向文件的访问权限。

## 公共契约

| API / type | 当前职责 | 明确不做 |
| --- | --- | --- |
| `PathUri::parse` | 校验并 canonicalize `file:` URI | 不接受其他 scheme、query、fragment、port 或 credentials |
| `from_absolute_path` / `to_host_path` | 当前 host absolute path 的 lossless round trip | 不把 foreign Windows path 映射成 POSIX path，反之亦然 |
| `from_native_path` | 按显式 `PathConvention` 解析 POSIX、drive 或 UNC path | 不访问文件系统 |
| `basename` / `parent` / `ancestors` / `join` | 跨 host lexical path operations | 不解析 symlink、case alias 或 Unicode filesystem normalization |
| `starts_with` / `relative_path_from` | 按 authority 与 URI segment 做 containment | encoded `/` 或 Windows `\` fail closed |
| `PathConvention` | 显式选择 POSIX/Windows grammar | 不代表一台具体机器或授权环境 |

Serde 和 `TS` 将 `PathUri` 表示为 canonical URI string。Windows drive letter 统一为大写，
`file://localhost/...` 统一为无 authority 的本地 URI。无法普通 URL 表示的 host-native path
使用保留的 opaque URI；opaque URI 只能在相同 host convention 上恢复，且除自身相等外不参与
lexical containment。

## 文件与内部所有权

| 文件 / private symbol | Ownership |
| --- | --- |
| `validation.rs::validated_file_url` | scheme、metadata、NUL、localhost 与 drive canonicalization |
| `validation.rs::decode_opaque_path_uri` | opaque namespace 的 canonical base64 验证 |
| `native.rs::parse_native_path` | 显式 POSIX/Windows absolute path parsing |
| `native.rs::from_segments` | `.` / `..` lexical normalization 与 root anchor |
| `native.rs::render_native_path` | foreign convention display，不依赖 current host |
| `operations.rs::containment_segments` | encoded-separator fail-closed boundary |

```text
PathUri::parse
  → Url::parse
  → validated_file_url

PathUri::from_native_path
  → native::parse_native_path
  → native::from_segments
  → validated_file_url

PathUri::starts_with / relative_path_from
  → operations::containment_segments
  → exact authority + segment comparison
```

如果实现开始 canonicalize 真实文件、跟随 symlink、应用 workspace grant，表示 ownership 已经
漂移到 `zeta-utils-path`、`zeta-sandboxing` 或具体 filesystem domain。Rust `PathUri` 若开始
接受任意 URI scheme，则必须先与 Desktop 通用 `URI` contract 统一，而不是私自扩张。

## 集成

- local-only API 可以继续使用 `Path`/`PathBuf`；
- workspace-relative RPC 继续使用相对 path，由 Rust authority 执行 root confinement；
- absolute file identity 跨 JSON/RPC、remote executor 或不同 OS 时使用 `PathUri`；
- 收到 `PathUri` 不等于获得读写授权，consumer 必须另行验证 capability/workspace boundary；
- domain 若需要忽略 fragment 等替代 identity，必须显式选择；`PathUri` 本身拒绝 fragment。

## 测试与修改影响

```text
cargo test -p zeta-utils-path-uri
bazel test //zeta-rs/utils/path-uri:path-uri-unit-tests
```

`path_uri_tests.rs` 覆盖 wire canonicalization、POSIX/Windows/UNC、serde、parent/join、
containment、encoded separator、host round trip 与 non-UTF-8 fallback。修改 URI string
spelling、normalization 或 serde 时，必须同步检查 Desktop `URI` fixtures 和未来 protocol
schema；修改 containment 时还要检查 sandbox/file-system consumer 的领域测试。

## 当前限制与扩展点

- Current：crate 已实现但尚无 production consumer。
- Current：只接受 `file:`，不表示 HTTP、MCP resource 或 editor fragment URI。
- Current：lexical identity 大小写敏感；filesystem-specific case comparison 属于 consumer。
- Current：opaque fallback 不支持 parent/join/relative traversal。
- Extension point：remote filesystem 或 executor protocol 可以采用 `PathUri`，前提是同时定义
  environment identity、授权、foreign-path failure 和 conversion boundary。
