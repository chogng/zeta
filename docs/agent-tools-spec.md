# Agent 工具规格

> 状态：Accepted（2026-08-03）
> 定位：[`agent-harness-design.md`](agent-harness-design.md) §5 工具面的实现规格：逐工具的
> JSON schema、**描述正文（模型可见的实际英文文本）**、参数校验规则、错误文案、输出格式与
> 限幅、capability 注记；附录 A 是系统提示词的工具指导与输出风格扩写正文（M0 时移入
> `zeta-prompts` 模板并 bump revision）。
>
> 工具的三层契约（定义/绑定/执行接口）归 [`tools.md`](tools.md)；审批/沙箱/升级语义归
> [`core.md`](core.md) §11。本文写"每个工具具体长什么样"。描述与错误文案是模型可见提示词
> 的一部分：修改需要与 system prompt 同级 review，并跑
> [`agent-harness-design.md` §14](agent-harness-design.md#14-评测) 的评测对比。

## 1. Schema 约定

所有工具 schema 按**最严格 provider 交集**编写，同一份 schema 直接用于 Anthropic
`input_schema` 与 OpenAI strict 模式：

1. 顶层必须是 `{"type": "object"}`；
2. `additionalProperties: false`；
3. **所有**属性进 `required`；可选性用类型并集表达：`{"type": ["string", "null"]}`；
4. 只用 `string` / `number` / `integer` / `boolean` / `array` / `object` / `enum`；
   顶层不用 `oneOf` / `anyOf` / `allOf`；
5. 每个参数带 `description`；数值参数写明单位与默认值；
6. 工具名用 `snake_case`，动词开头或名词短语，与描述首句一致。

错误文案原则：错误信息是模型的下一条输入——必须**可自纠**（说明哪个参数错、给出正确
形态或下一步动作），不写只有人类能懂的内部错误。

## 2. shell

| | |
| --- | --- |
| 状态 | 已接入（`zeta-shell-command` + `local_tools.rs`），schema 需按本节收敛 |
| 执行 | 沙箱 + 审批走 `zeta-policy`；执行上限 256 KiB / 默认 30s（已有） |
| 模型侧限幅 | 30 KiB，头尾各半，中间标注 `[... N bytes truncated ...]` |

**description（模型可见）：**

```text
Executes a shell command in the workspace and returns its combined stdout and
stderr with the exit code.

Usage notes:
- Use dedicated tools instead of shell equivalents when available: read_file
  instead of cat, grep instead of grep/rg, glob instead of find, edit or
  apply_patch instead of sed -i. Dedicated tools produce better results.
- Commands run with the workspace root as the default working directory; state
  such as environment variables does not persist between calls. Chain dependent
  steps with && in a single call.
- Long output is truncated from the middle; rerun with a narrower command (e.g.
  pipe through head/tail) if you need the omitted part.
- Never run interactive commands that wait for input (e.g. git rebase -i,
  npm init without -y); they will hang until the timeout.
```

**parameters：**

```json
{
  "type": "object",
  "properties": {
    "command": {
      "type": "string",
      "description": "The shell command to execute, run via the login shell with -c."
    },
    "timeout_ms": {
      "type": ["integer", "null"],
      "description": "Timeout in milliseconds. Default 30000, maximum 600000."
    },
    "working_directory": {
      "type": ["string", "null"],
      "description": "Absolute path to run in. Defaults to the workspace root. Must stay inside the workspace."
    }
  },
  "required": ["command", "timeout_ms", "working_directory"],
  "additionalProperties": false
}
```

**校验与错误文案：**

| 情形 | 结果（`is_error: true` 的 Tool Result 文本） |
| --- | --- |
| `command` 为空/全空白 | `command must not be empty` |
| `working_directory` 在 workspace 外 | `working_directory is outside the workspace: {path}. Use a path under {root}` |
| `timeout_ms` 超上限 | `timeout_ms exceeds the maximum of 600000` |
| 超时 | `command timed out after {n} ms. Partial output:\n{截断输出}` |
| 非零退出 | 正常结果：输出 + `exit code: {n}`（非零退出不是工具错误） |

**输出格式：**合并流文本 + 末行 `exit code: {n}`。沙箱拒绝走 `SandboxDenialOutput` 结构化
路径（[`core.md`](core.md) §11），不进本表。

## 3. read_file

| | |
| --- | --- |
| 状态 | `file-system-tool` 已有 `read` 操作，需改造为独立工具 + 行号/offset/limit/图片 |
| 模型侧限幅 | 默认 2000 行；单行 > 2000 字符截断并标注 |

**description：**

```text
Reads a file from the workspace and returns its content with line numbers.

Usage notes:
- Returns at most 2000 lines starting from `offset` (1-based). The last line
  of a truncated read says how many lines remain; call again with a larger
  offset to continue.
- Lines longer than 2000 characters are truncated with a marker.
- Image files (png, jpg, gif, webp) are returned as viewable images.
- You must read a file before editing or overwriting it.
- Prefer reading whole files (omit offset/limit) unless the file is too large.
```

**parameters：**

```json
{
  "type": "object",
  "properties": {
    "path": {
      "type": "string",
      "description": "Absolute path to the file to read."
    },
    "offset": {
      "type": ["integer", "null"],
      "description": "1-based line number to start from. Defaults to 1."
    },
    "limit": {
      "type": ["integer", "null"],
      "description": "Maximum lines to return. Defaults to 2000."
    }
  },
  "required": ["path", "offset", "limit"],
  "additionalProperties": false
}
```

**校验与错误文案：**

| 情形 | 文案 |
| --- | --- |
| 文件不存在 | `file not found: {path}`；若同目录有相近命名，附 `did you mean {candidate}?` |
| 是目录 | `{path} is a directory. Use glob to list its files` |
| 二进制且非图片 | `{path} is a binary file and cannot be displayed as text` |
| 超出 workspace | `path is outside the workspace: {path}` |
| 空文件 | 正常结果：`(file is empty)` |

**输出格式：**`cat -n` 风格行号 + 制表符；截断尾注
`[... {n} more lines, continue with offset={next}]`。durable Tool Result 存完整读取结果的
限幅版（执行侧即模型侧，读取本身无副作用）。

## 4. write_file

| | |
| --- | --- |
| 状态 | 新增（`file-system-tool` 扩展） |
| capability | 写路径进入 `ActionReviewRequest`，沙箱/审批按路径判定 |

**description：**

```text
Creates or overwrites a file with the given content.

Usage notes:
- Overwriting an existing file you have not read in this conversation fails;
  read it first.
- Prefer edit (or apply_patch) for modifying existing files; use write_file
  for new files or full rewrites you have already read.
- Parent directories are created automatically.
- Never proactively create documentation files unless explicitly requested.
```

**parameters：**

```json
{
  "type": "object",
  "properties": {
    "path": {
      "type": "string",
      "description": "Absolute path of the file to write."
    },
    "content": {
      "type": "string",
      "description": "Full content to write. The previous content is replaced entirely."
    }
  },
  "required": ["path", "content"],
  "additionalProperties": false
}
```

**校验与错误文案：**

| 情形 | 文案 |
| --- | --- |
| 覆盖未读过的既有文件 | `{path} exists but has not been read in this conversation. Read it first, or choose a new path` |
| 超出 workspace | `path is outside the workspace: {path}` |
| 目标是目录 | `{path} is a directory` |

"读过"判定：本 Thread durable history 中存在该路径成功的 `read_file` Tool Result，且其后
无该文件的外部修改通知 reminder。此校验由工具实现基于 Thread 提供的已读路径集完成（Core
在 prepare 阶段传入，工具不读 Thread store——遵守 [`tools.md`](tools.md) 依赖边界）。

## 5. edit（anthropic / google / 默认 profile）

| | |
| --- | --- |
| 状态 | 新增（`file-system-tool` 扩展） |
| 核心不变量 | `old_string` 唯一命中，否则拒绝——这条校验挡住大部分错误编辑 |

**description：**

```text
Performs an exact string replacement in a file.

Usage notes:
- You must read the file first; the edit fails otherwise.
- old_string must match the file content exactly, including whitespace and
  indentation, and must identify a unique location. If it matches more than
  one location, extend it with surrounding lines until unique, or set
  replace_all to true to change every occurrence.
- Do not include line-number prefixes from read_file output in old_string.
- For moving or renaming files use shell with git mv; for full rewrites use
  write_file.
```

**parameters：**

```json
{
  "type": "object",
  "properties": {
    "path": {
      "type": "string",
      "description": "Absolute path of the file to modify."
    },
    "old_string": {
      "type": "string",
      "description": "Exact text to replace. Must be unique in the file unless replace_all is true."
    },
    "new_string": {
      "type": "string",
      "description": "Replacement text. Must differ from old_string."
    },
    "replace_all": {
      "type": ["boolean", "null"],
      "description": "Replace every occurrence. Defaults to false."
    }
  },
  "required": ["path", "old_string", "new_string", "replace_all"],
  "additionalProperties": false
}
```

**校验与错误文案：**

| 情形 | 文案 |
| --- | --- |
| 未读过该文件 | `{path} has not been read in this conversation. Read it first` |
| 0 命中 | `old_string not found in {path}. Re-read the file: the content may differ from what you expect (check whitespace and indentation)` |
| ≥2 命中且非 replace_all | `old_string matches {n} locations in {path}. Extend it with more surrounding context to make it unique, or set replace_all to true` |
| old == new | `new_string must differ from old_string` |
| 读后被外部修改 | `{path} changed on disk after your last read. Read it again before editing` |

**输出格式：**成功返回替换处 ±4 行的带行号片段（模型自查 + UI 可渲染 diff）。

## 6. apply_patch（openai profile）

| | |
| --- | --- |
| 状态 | `zeta-apply-patch` crate 已存在，需接入 profile |
| 格式 | V4A envelope（OpenAI 系模型的训练格式，原样采用不发明方言） |

**description：**

```text
Applies a patch to create, delete, or modify files using the V4A format:

*** Begin Patch
*** Update File: path/to/file
@@ context line
-removed line
+added line
*** End Patch

Usage notes:
- Use *** Add File: / *** Delete File: / *** Update File: headers; an Update
  may include *** Move to: for renames.
- Context lines must match the current file content exactly. Read the file
  first if you are not certain.
- Keep patches minimal and focused; unrelated files must not appear.
```

**parameters：**

```json
{
  "type": "object",
  "properties": {
    "input": {
      "type": "string",
      "description": "The full patch text, starting with *** Begin Patch and ending with *** End Patch."
    }
  },
  "required": ["input"],
  "additionalProperties": false
}
```

**校验与错误文案：**格式解析失败 →
`invalid patch: {原因} at line {n}. The patch must start with *** Begin Patch and use *** Update File: / *** Add File: / *** Delete File: headers`；
上下文行不匹配 →
`patch does not apply to {path}: context mismatch near "{片段}". Read the current file content and regenerate the patch`。
成功输出各文件的变更摘要（`M path (+a/-b)`）。

## 7. glob

| | |
| --- | --- |
| 状态 | 新增（`zeta-file-search` 已有路径索引，补 glob 语义） |
| 限幅 | 100 条，按修改时间降序 |

**description：**

```text
Finds files by glob pattern, sorted by most recently modified.

- Supports patterns like "**/*.rs" or "src/**/*.test.ts".
- Returns at most 100 paths; narrow the pattern if truncated.
- Use grep to search file contents; use glob to find files by name.
```

**parameters：**

```json
{
  "type": "object",
  "properties": {
    "pattern": {
      "type": "string",
      "description": "Glob pattern to match file paths against."
    },
    "path": {
      "type": ["string", "null"],
      "description": "Directory to search in. Defaults to the workspace root."
    }
  },
  "required": ["pattern", "path"],
  "additionalProperties": false
}
```

**错误文案：**非法 pattern → `invalid glob pattern: {原因}`；无命中 → 正常结果
`no files match {pattern}`（不是错误）。截断尾注 `[{total} matches, showing first 100]`。

## 8. grep

| | |
| --- | --- |
| 状态 | 新增封装（`RipgrepExecutable` 已被 App Server 发现和管理） |
| 限幅 | 100 条命中；单行 > 500 字符截断 |

**description：**

```text
Searches file contents with a regular expression (ripgrep syntax).

- Full regex support, e.g. "fn\\s+resolve" or "TODO|FIXME".
- Results are file:line:content, capped at 100 matches; narrow with `glob`
  or `path` if truncated.
- Literal braces or dots must be escaped: use "foo\\.bar" to match "foo.bar".
- Always use this tool instead of invoking grep or rg through shell.
```

**parameters：**

```json
{
  "type": "object",
  "properties": {
    "pattern": {
      "type": "string",
      "description": "Regular expression to search for (ripgrep syntax)."
    },
    "path": {
      "type": ["string", "null"],
      "description": "File or directory to search. Defaults to the workspace root."
    },
    "glob": {
      "type": ["string", "null"],
      "description": "Restrict to files matching this glob, e.g. \"*.rs\"."
    },
    "case_insensitive": {
      "type": ["boolean", "null"],
      "description": "Case-insensitive search. Defaults to false."
    }
  },
  "required": ["pattern", "path", "glob", "case_insensitive"],
  "additionalProperties": false
}
```

**错误文案：**正则非法 → 透传 rg 的错误信息 + `escape literal characters like . ( ) { } with a backslash`；
无命中 → 正常结果 `no matches`。默认遵守 .gitignore（rg 行为）。

## 9. update_plan

| | |
| --- | --- |
| 状态 | 新增薄工具，durable 提交 `ThreadItem::Plan`（protocol 已有该 Item） |
| 组装 | assembler 停止跳过 Plan：**最新一条** Plan 注入当前窗口，旧 Plan 被压缩吸收 |

**description：**

```text
Records or updates your plan for a multi-step task.

- Use for tasks that need 3 or more distinct steps; skip it for trivial work.
- Keep exactly one step in_progress at a time. Mark a step completed as soon
  as it is done; update the plan when scope changes rather than following a
  stale plan.
- Steps are short imperative phrases ("Fix parser offset bug"), not essays.
```

**parameters：**

```json
{
  "type": "object",
  "properties": {
    "plan": {
      "type": "array",
      "description": "The full plan, replacing any previous plan.",
      "items": {
        "type": "object",
        "properties": {
          "step": { "type": "string", "description": "Short imperative description." },
          "status": { "type": "string", "enum": ["pending", "in_progress", "completed"] }
        },
        "required": ["step", "status"],
        "additionalProperties": false
      }
    }
  },
  "required": ["plan"],
  "additionalProperties": false
}
```

**校验与错误文案：**空 plan → `plan must contain at least one step`；
多个 in_progress → `keep at most one step in_progress (found {n})`；
step 为空 → `every step needs a non-empty description`。成功输出 `plan updated`。

## 10. MCP 元工具（检索式模式，[harness §6](agent-harness-design.md#6-工具注册时机)）

### 10.1 search_tools

```text
Searches the available MCP tools by keyword and returns matching tool
definitions. Use when the task needs a capability not covered by your core
tools (e.g. a service integration). Found definitions become callable via
call_mcp_tool.
```

```json
{
  "type": "object",
  "properties": {
    "query": { "type": "string", "description": "Keywords describing the needed capability." }
  },
  "required": ["query"],
  "additionalProperties": false
}
```

返回 ≤ 5 个定义（name + description + schema），以 reminder 形式进入历史
（[harness §4.4](agent-harness-design.md#44-动态注入append-only-reminder)），不修改 tools
数组。无命中 → `no MCP tools match "{query}". Available servers: {list}`。

### 10.2 call_mcp_tool

```json
{
  "type": "object",
  "properties": {
    "tool": { "type": "string", "description": "Fully qualified name server__tool from search_tools results." },
    "arguments": { "type": "object", "description": "Arguments matching the tool's schema.", "additionalProperties": true }
  },
  "required": ["tool", "arguments"],
  "additionalProperties": false
}
```

未先 search 的调用 → `unknown tool {name}; use search_tools first`。参数校验错误透传 MCP
server 的 schema 错误。执行、审批与结果转换沿用 [`tools.md`](tools.md) §10 的 MCP 适配器。

## 附录 A：系统提示词扩写正文

以下为 `SYSTEM_PROMPT` 的工具指导与输出风格扩写段，M0 时追加进
`zeta-rs/prompts/templates/system/`（工具指导按 profile 分模板）并 bump revision。现有
base.md 的身份/优先级/防注入/工作行为四段保留在前。

### A.1 工具指导（共享段）

```text
## Tool usage

- Search before you read, read before you edit: locate code with grep and
  glob, read the relevant files, then make changes. Do not edit code you have
  not seen.
- Use the dedicated tools (read_file, grep, glob, edit) instead of their shell
  equivalents (cat, rg, find, sed). Use shell for builds, tests, git, and
  anything without a dedicated tool.
- Prefer several small, verifiable changes over one large speculative change.
- After a code change, verify it with the narrowest relevant check (the
  affected test, a typecheck, a targeted build) before moving on. Do not claim
  success without having verified.
- When a command or tool fails twice with the same error, stop repeating it.
  Diagnose, try a different approach, or report the blocker.
- For tasks with 3 or more distinct steps, maintain a plan with update_plan
  and keep it current.
```

### A.2 工具指导（profile 差异段）

edit profile（anthropic / google / 默认）：

```text
- Modify files with edit using an exact unique snippet; extend old_string
  with surrounding lines when the match is ambiguous. Use write_file only for
  new files or full rewrites of files you have read.
```

apply_patch profile（openai）：

```text
- Modify files with apply_patch. Keep each patch focused on one logical
  change; regenerate the patch from current file content when context lines
  fail to match.
```

### A.3 输出风格（共享段）

```text
## Output style

- Your replies render in a developer-facing client. Be concise and direct:
  answer first, qualifications after, no filler ("Great!", "Certainly").
- Reference code as `path:line` so the user can jump to it. Use fenced code
  blocks only for code, commands, or file content - not for emphasis.
- After completing a task, summarize what changed and what you verified in a
  few sentences. Report failures plainly with the relevant output; never
  claim an unverified result.
- Ask the user only when a decision genuinely belongs to them (destructive
  actions, ambiguous requirements with materially different readings);
  otherwise pick the reasonable default and note the assumption.
```

## 附录 B：修改清单

修改本文任何工具的 schema、描述或错误文案时同步：

1. 对应实现 crate 的 schema/文案常量与测试；
2. `zeta-prompts` 中引用该工具名的指导段（附录 A）与 revision；
3. [`agent-harness-design.md` §14](agent-harness-design.md#14-评测) 的组装快照 fixture；
4. 评测跑 T1/T2 对比，回归超阈值不合入。
