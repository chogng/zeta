# `zeta-file-identity`

`zeta-file-identity` 拥有从已打开文件读取稳定文件身份和硬链接数量的平台边界。信任决策需要把
后续读取绑定到同一文件系统对象时，调用方使用 `FileInformation::from_file`；打开受控路径前
只需检查时，使用 `FileInformation::from_path`。

公共契约只暴露 `FileIdentity`、`FileInformation` 及其比较和链接数量访问器：

- Unix 通过 `MetadataExt` 读取设备号、inode 和链接数量；
- Windows 通过一个私有 FFI 边界调用 `GetFileInformationByHandle`，把卷序列号和文件索引
  映射为相同的领域中立身份；
- 操作系统错误保持为 `io::Error`；
- 不支持的平台明确失败，不退化为路径字符串比较。

```text
受控路径
└── FileInformation::from_path
    └── File::open
        └── FileInformation::from_file
            └── platform::inspect
                ├── Unix MetadataExt
                └── Windows GetFileInformationByHandle
```

本 crate 不规范化路径、不拒绝符号链接、不决定是否允许硬链接、不监听修改，也不使用领域专用
标志打开文件。这些义务仍属于调用方。把路径策略、Skill 语义或沙箱行为放进这里，意味着架构
所有权已经漂移。

Skills 目录是当前消费者：它拒绝多链接清单，并验证路径仍然指向已扫描内容所属的文件句柄。
跨 crate 的所有权与信任语义见 [`../../docs/skills.md`](../../docs/skills.md)。

当前限制：只支持 Zeta 使用的 Unix 与 Windows 主机系列。新增平台必须能够同时返回稳定身份和
链接数量；退化为路径拼写或静默省略链接信息不符合本契约。

## 验证

```bash
cargo test --manifest-path zeta-rs/Cargo.toml -p zeta-file-identity
cargo clippy --manifest-path zeta-rs/Cargo.toml \
  -p zeta-file-identity --all-targets --no-deps -- -D warnings
```
