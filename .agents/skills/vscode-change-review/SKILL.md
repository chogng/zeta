---
name: vscode-change-review
description: Review new commits in the checked-out VS Code repository since the recorded checkpoint, decide which changes affect Zeta, and record review and alignment progress without modifying product source. Use for daily or periodic VS Code upstream audits; do not use to implement the resulting Zeta changes.
---

# VS Code 提交增量审查

比较 `../vscode` 两个提交之间的真实改动，找出可能需要应用到 Zeta 的变化。该 skill 只读取 VS Code 和 Zeta 源码，只允许更新自身的 [checkpoint.json](checkpoint.json)；不得修改、生成、删除或格式化任何产品源码。

## 两个检查点

- `reviewed`：已经逐提交、逐差异审查到的 VS Code 提交。
- `aligned`：用户已经确认相关 Zeta 改动全部完成或无需应用的 VS Code 提交。

审查完成可以推进 `reviewed`，不能自动推进 `aligned`。只要目标提交之前仍有待应用或待决定事项，就不能把 `aligned` 推进到该提交。

## 选择范围

1. 读取 `checkpoint.json` 并运行 `node .agents/skills/vscode-change-review/scripts/review-vscode-changes.mjs --check`。
2. 用户要求远端最新状态时，运行 `git -C ../vscode fetch origin main`，目标使用 `origin/main`；只要求当前检出状态时使用 `HEAD`。不得 checkout、pull、reset 或修改 VS Code 工作树。
3. 起点默认使用 `reviewed.commit`。首次运行没有起点时必须让用户指定可信基线，不能把当前提交直接记作已审查。
4. 使用脚本生成提交与文件清单：

```powershell
node .agents/skills/vscode-change-review/scripts/review-vscode-changes.mjs --to=origin/main
```

需要机器可读结果时添加 `--json`；临时指定起点时使用 `--from=<commit>`。

## 审查方法

对范围内每个提交读取完整补丁、提交说明和相关测试，不只看文件名或提交标题。批量按 Zeta 对应层归组，然后逐组确认：

- 公开 API、职责归属、生命周期、调用顺序、副作用、错误语义或可观察行为是否变化；
- Zeta 是否存在对应文件、API、调用方和测试；
- 变化应应用到 Zeta、无需应用，还是需要用户决定；
- 后续实现应使用哪些 Zeta 路径和定向测试。

VS Code 的重构、修复和测试变化都要检查。纯文案、构建或产品差异也必须确认无关后才能略过。禁止复制、翻译或轻微改写上游实现与测试。

## 记录结果

完整审查目标范围后才更新 `checkpoint.json`：

- 把 `reviewed.commit` 写为本次目标提交，并记录本次日期；
- 需要应用的变化写入 `pending`，包含稳定 `id`、引入提交、VS Code 路径、Zeta 路径、`apply` 或 `decision` 类型以及简短原因；
- 已确认无需应用的变化不进入 `pending`，但必须在本次回复中按提交汇总理由；
- 合并已存在的待办，不重复创建同一问题，也不删除尚未解决的旧待办；
- 不修改 `aligned`。

用户明确确认对应改动已经完成后，重新核对从当前 `aligned` 到目标提交的全部待办。只有该范围没有未解决项时，才可更新 `aligned` 并再次运行 `--check`。

提交审查结束后不得自动调用 API 对齐流程。只有用户另行要求实施时，才把 `pending` 中选定事项交给 `vscode-api-alignment`。

## 完成输出

报告本次提交范围、审查提交数、需要应用、无需应用、需要决定和仍未解决的历史待办。没有新提交时也要报告当前 `reviewed`、`aligned` 和待办数量，不制造空检查记录。
