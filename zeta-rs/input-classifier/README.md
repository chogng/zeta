# `zeta-input-classifier`

`zeta-input-classifier` 拥有本地 Shell/Agent 自动路由的完整决策管线：输入 parser、最近提交历史、
Shell token 语义、工作区/PATH 解析、确定性规则、BERT-Tiny v3 模型与 tokenizer、模型失败后的 fallback。
产品宿主只提供当前工作目录、当前路由和会话位置，不得复制标签顺序、阈值或词典。分类结果不做
命令风险判断，也不授权执行。

## 公共契约

| Symbol | 职责 | 接入要求 |
| --- | --- | --- |
| `InputClassifier` | 持有工作目录和 PATH 快照，执行整条分类管线 | 一个 Composer 持有一个实例；工作区切换时调用 `set_working_directory` |
| `InputClassificationContext` | 提供 `current_route` 和 `InputConversation` | 只用于自动模式；显式模式必须优先 |
| `InputConversation` | 区分普通输入与 Agent 回复后的短追问 | Agent Turn 完成后设为 `AgentFollowUp`，新提交或失败后复位 |
| `InputHistoryEntry` | 提供按时间排序的 Shell 命令和 Agent prompt | Snapshot 必须按 Turn 顺序重建；command-not-found 不进入 Shell 历史 |
| `InputClassification` | 返回路由、置信度与决策来源 | 置信度只用于诊断，不得作为执行授权 |
| `start_background_warmup` | 后台解码一次内嵌模型和 tokenizer | 创建输入界面时调用，避免首次按键承担加载延迟 |

Zeterm 只把 `InputRoute` 映射为 Composer 模式，并把真实 Turn 生命周期投影成
`InputConversation` 和按顺序排列的 `InputHistoryEntry`。Shell parser、相似度计算、PATH/manifest
检查和 fallback 不再属于产品宿主。

## 执行路径

```text
InputClassifier::classify
  → parse_query_into_tokens
  → contextual rules
      empty / current-Agent NL one-off / Agent follow-up
  → InputHistory
      newest Shell command / Agent prompt close match, cutoff 0.9
  → NL one-off / Shell allowlist
  → ShellContext::analyze
      parse_shell_tokens
      executable + builtin + workspace runner + subcommand/flag/path descriptions
      strict token threshold
  → EmbeddedClassifier
      Tokenizer::encode_fast
      candle_onnx::simple_eval
      temperature scaling + softmax
  → ordinary error: current_route
  → panic: dictionary/stemmer heuristic fallback
```

Shell token 阈值采用 Warp 当前严格方案：所有解析 token 都有 Shell 语义时直接判为 Shell；少于
3 个自然语言 token 时，只要最后一个 command-position token 是已知命令也判为 Shell。其他输入交给模型，避免把
`git status 是做什么的` 这类 command-prefix 问句提前截成命令。

`natural_language.rs` 的 fallback 会分别尝试排除和包含未完成的末 token，并按 1.0、0.8、
0.6 的长度阈值使用 English stems、developer terms 和 command-overlap 词典打分。这三份词典为
Zeta 独立构建，没有导入或变换 Warp 的词表。英文候选数据来自 MIT 许可的 TextBlob bundled
spelling-frequency data，并经过 frequency threshold 和 Snowball stemming；developer terms 和
command-overlap 是为 Zeta 独立整理的集合。TextBlob 许可文本位于
[`dictionaries/THIRD_PARTY_NOTICES.txt`](dictionaries/THIRD_PARTY_NOTICES.txt)。词典作为本 crate
的数据内嵌进二进制，不引入单独运行时。

## 模型资产与失败语义

当前发布来自 `chogng/zeta-classifier` 的 `models/bert_tiny_v3`：

- `bert_tiny_v3_candle.onnx`：Candle 0.9 专用 FP32、opset 16；
- `bert_tiny_tokenizer.json`：训练分区适配 tokenizer；
- `metadata.json`：标签、温度、算子与摘要；
- 输出 index 0 为 Shell、index 1 为 Agent；softmax 前温度为 `1.6894922825552194`。

模型和 tokenizer 通过 `include_bytes!` 固定进二进制，不在运行时下载。`EmbeddedClassifier` 是
模型、tokenizer 和 CPU device 的唯一 owner。模型或 tokenizer 初始化失败会直接启用完整
`HeuristicFallback`；初始化成功后的编码、推理或输出校验错误返回 `CurrentRouteFallback`，保留用户
正在使用的模式；Candle panic 会被隔离，并在当前进程永久改走 `HeuristicFallback`。官方
`candle-onnx` 的构建脚本需要系统 `protoc`，产品运行时不需要。

更新模型时必须一起更新 ONNX、tokenizer、`metadata.json`、摘要常量、标签解释、温度和概率基线
测试，不能只替换其中一个文件。

## 内部 owner 与修改影响

- `classifier::classify_with_model` 固定决策顺序与失败分支；改动会影响所有自动模式消费者。
- `history::InputHistory` 拥有 0.9 相似度门槛和“最新匹配胜出”语义；宿主只提供有序事实。
- `shell::ShellContext` 拥有命令和 token 描述；把它移回 Zeterm 会造成产品间行为分叉。
- `rules` 只放低风险、确定性的上下文和 allowlist 短路；模糊规则不得绕过模型。
- `model::EmbeddedClassifier` 绑定模型图、tokenizer、标签和温度；任一资产变更都需要概率测试。
- `natural_language::classify_with_fallback_heuristic` 只在模型不可用或 panic 后接管；它使用 Zeta
  自有词典，不应成为正常主路径。`natural_language` 保持为本 crate 的私有模块：产品消费者必须
  调用完整 `InputClassifier`，不得直接依赖词典分数绕过历史、Shell evidence 和模型。

当前没有第二个需要“原始自然语言分数”而非 Shell/Agent 路由的后端无关消费者，因此不拆独立
natural-language-detection crate。只有出现这样的真实消费者，并且能定义不依赖 `InputRoute`、
`ShellTokenSnapshot` 和 classifier fallback 阈值的纯打分契约时，才应把词典和 scorer 一起抽出；
路由顺序与失败语义仍由 `zeta-input-classifier` 拥有。

## 验证

```bash
cargo test -p zeta-input-classifier
cargo clippy -p zeta-input-classifier --all-targets -- -D warnings
```

测试覆盖 parser、历史冲突、严格 token 阈值、工作区命令、自然语言词典、follow-up、普通模型错误、模型 panic、
模型路由样例、资产 SHA 和 Candle FP32 概率基线。当前模型仍可能误判自由措辞，例如
`chmod 755 是什么意思`；用户显式选择始终覆盖自动分类。
