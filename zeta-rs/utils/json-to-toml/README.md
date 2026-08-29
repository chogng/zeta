# zeta-utils-json-to-toml

`zeta-utils-json-to-toml` 负责把调用边界收到的 `serde_json::Value` 转成可交给 TOML 配置层的 `toml::Value`：

1. JSON object 和 array 递归保持容器结构，普通 scalar 映射到对应 TOML scalar；
2. TOML 没有 null，JSON null 固定映射为空字符串；超出 TOML `i64` 整数范围的 JSON 数字按浮点数处理，无法表示时保留十进制字符串；
3. crate 只拥有值转换规则，不负责解析文本、合并配置、校验配置字段或写入文件。

公共 API 只有 `json_to_toml`，接收 owned JSON value 并返回 owned TOML value。修改 null、数字或递归容器规则时，运行：

```text
just test zeta-utils-json-to-toml
bazel test //zeta-rs/utils/json-to-toml:json-to-toml-unit-tests
```
