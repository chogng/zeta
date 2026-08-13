# `zeta-input-classifier`

`zeta-input-classifier` 拥有本地 Shell/Agent 输入分类的模型资产、确定性短语规则、tokenizer
绑定、ONNX 推理和标签解释。产品宿主只消费 `InputRoute`，不得复制模型标签顺序或在运行时从
网络下载模型。本 crate 不判断命令风险，也不拥有批准、执行或沙箱策略。

## 公共契约与执行路径

| Symbol | 职责 | 接入要求 |
| --- | --- | --- |
| `classify_input` | 按 Heuristics、宿主证据、BERT 的顺序返回路由 | 仅用于自动模式；用户显式选择必须优先 |
| `ShellCommandEvidence` | 接收宿主基于 PATH 和工作区得到的高置信命令证据 | 不能把长句中出现可执行文件当作高置信证据 |
| `start_background_warmup` | 在后台解码一次内嵌资产 | 产品创建输入界面时调用，避免首次按键承担加载延迟 |
| `InputClassification` | 携带路由、置信度和来源 | 置信度只用于诊断，不得当作执行授权 |
| `InputRoute` | 稳定的产品无关路由结果 | 宿主负责映射到自己的 Composer 模式 |

```text
classify_input
  → classify_deterministic_input
  → ShellCommandEvidence
  → EmbeddedClassifier::load（进程内一次，可由 start_background_warmup 提前触发）
  → Tokenizer::encode_fast
  → candle_onnx::simple_eval
  → temperature scaling + softmax
  → InputRoute
```

`EmbeddedClassifier` 是模型、tokenizer 和 CPU device 的进程内 owner。`MODEL_BYTES` 与
`TOKENIZER_BYTES` 在构建时内嵌，`CALIBRATION_TEMPERATURE` 和标签顺序固定到同一模型发布。
出现运行时下载、由产品宿主解释 logits，或 tokenizer 与模型独立更新，均表示 ownership 已漂移。

## 资产与失败语义

当前发布来自 `chogng/zeta-classifier` 的 `models/bert_tiny_v3`：

- 模型：`bert_tiny_v3_candle.onnx`，opset 16，SHA-256 由 `MODEL_SHA256` 固定；
- tokenizer：训练分区适配的 `bert_tiny_tokenizer.json`，SHA-256 由
  `TOKENIZER_SHA256` 固定；
- 输入：`input_ids` 与 `attention_mask`，最多 128 tokens；
- 输出：index 0 为 Shell，index 1 为 Agent；
- softmax 前使用 FP32 发布记录的温度 `1.6894922825552194`。

模型或 tokenizer 加载、编码、推理和输出校验失败都会返回错误；Candle 推理 panic 会被隔离并在
当前进程永久关闭后续模型调用。Zeterm 在失败时保留当前模式；
PATH 和工作区 manifest 只作为进入模型前的高置信 `ShellCommandEvidence`。分类结果不能授权高风险
命令执行。官方 `candle-onnx` 的构建脚本需要系统提供 `protoc`，这是构建依赖，不进入产品运行时。

更新模型时必须一起更新 ONNX、tokenizer、`metadata.json`、摘要常量、标签解释、温度和测试样例。
不能只替换其中一个文件。

## 测试与当前限制

```bash
cargo test -p zeta-input-classifier
cargo clippy -p zeta-input-classifier --all-targets -- -D warnings
```

- **Current**：明确自然语言前缀、Shell keyword 和 Shell syntax 先走 Heuristics；PATH/工作区提供高
  置信 Shell 证据；其余模糊输入才进入 BERT-Tiny。
- **Current**：CPU 上使用官方 Candle 0.9 执行内嵌 BERT-Tiny v3 Candle FP32 ONNX，不维护本地
  Candle fork。
- **Current**：模型只做 Shell/Agent 路由，不做危险性分类、批准或执行。
- **Current limitation**：模型仍可能误判自由措辞，例如 `chmod 755 是什么意思`；用户显式模式选择
  始终覆盖自动分类。
- **Current limitation**：v3 INT8 资产仍包含 Candle 0.9 尚未实现的 `DequantizeLinear` 算子，因此
  当前固定使用 Candle-compatible FP32；升级量化模型前必须先通过本 crate 的真实推理测试。
