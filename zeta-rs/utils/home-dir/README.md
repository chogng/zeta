# zeta-utils-home-dir

- `find_zeta_home` 统一解析全产品共享的 profile root：`ZETA_PROFILE_ROOT` 优先，否则使用 `<home>/.zeta`。
- 显式覆盖必须已存在且为目录，返回前会 canonicalize；默认目录不要求预先存在，所有成功结果都是 `AbsolutePathBuf`。
- 实现和测试位于 `src/lib.rs`、`src/home_dir_tests.rs`；修改环境变量、默认路径或失败语义后运行 `just test zeta-utils-home-dir`。
