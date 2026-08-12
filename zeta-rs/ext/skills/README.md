# `zeta-skills-extension`

Canonical cross-crate Skill semantics and staged work are documented in
[`docs/skills.md`](../../../docs/skills.md). The lower-level catalog and file contract remains in
[`zeta-skills`](../../skills/README.md).

`zeta-skills-extension` owns the agent-runtime orchestration around `zeta-skills`. It composes
built-in, user, and active Workspace sources; applies enablement and compatibility policy; freezes
explicit selections before Turn commit; reloads exact content at model-invocation safe points; and
contributes a bounded metadata catalog, exact Skill instructions, and the read-only `skills-read`
model tool through `zeta-extension-api`.

## Ownership

| Symbol | Responsibility |
| --- | --- |
| `SkillRuntime` | Catalog generations, source composition, exact activation, watcher lifecycle |
| `SkillActivationContributor` implementation | Map explicit `UserInput::Skill` selections to durable activations |
| `TurnInputContributor` implementation | Emit `catalog_prompt` metadata plus exact frozen activation fragments |
| `SkillToolContributor` | Register `SkillReadTool` without coupling the runtime to App Server |
| `SkillReadTool` | Resolve exact `source + name`, enforce enablement/compatibility, and return the bounded `SKILL.md` body |
| `SkillRuntimeEventSink` | Notify the installing host that the public catalog generation changed |

`zeta-skills` remains the lower-level file/catalog authority. Core invokes only generic extension
lifecycle contracts. App Server may provide configuration and event adapters and expose list DTOs,
but it must not select, activate, load, cache, or render Skill instructions.

The call path is:

```text
install
  -> TurnInputContributor::contribute
       -> catalog_prompt (metadata only, at most 8 KiB)
  -> ReadOnlyToolContributor::contribute
       -> SkillReadTool::execute
            -> SkillRuntime::activate_model_selected
                 -> SkillCatalog::activate
```

Manual selection and model selection share catalog validation and exact file loading, but they have
different durable shapes. Manual selection is frozen in `TurnAccepted` and contributed as a
`PromptFragment`. Model selection happens during the tool loop; the successful `skills-read` result
is durable model context for the next invocation. There is no backend keyword classifier.

Current limitations: Skill references/resources still have no model-facing rooted reader, and the
catalog prompt uses a fixed byte budget rather than a tokenizer-aware share of the selected model's
context window.
