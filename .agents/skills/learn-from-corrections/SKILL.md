---
name: learn-from-corrections
description: Capture durable, generalized instructions from corrections in the current conversation. Use when the user says “learn!”, asks to record a lesson, or wants feedback, a mistake, or a successful fix converted into a reusable learning in the most appropriate instruction file.
---

# Learn From Corrections

Convert explicit user corrections into concise, reusable instructions and persist them in the instruction file that owns the affected scope.

## Workflow

1. Inspect the recent conversation and identify:
   - the problem or mistake that occurred;
   - why it was a problem;
   - the correction, workaround, or user-provided fix; and
   - the broader rule that would prevent similar problems.
2. Generalize the rule. Avoid names, one-off values, temporary state, and details that only apply to the immediate change unless they express a reusable boundary.
3. Draft one learning in 1–4 sentences. Use direct, imperative language where practical. State the rule and its important rationale or exception; do not write a chronological postmortem.
4. Show the drafted learning to the user in the response, then persist that same wording.
5. Locate the most appropriate existing instruction file by scope and ownership:
   - prefer the nearest instruction file governing the affected project or directory;
   - use a more specific domain or skill instruction file when the learning belongs there;
   - use a broader instruction file only when the rule genuinely applies at that broader scope.
6. Add the learning under a `## Learnings` section. Create that section at the end of the selected file when it does not exist. Preserve unrelated instructions and surrounding formatting.
7. Before editing, check the selected file for an equivalent or overlapping learning. Refine or extend the existing item instead of adding a duplicate.
8. Use `apply_patch` for the edit, then verify the resulting section and report the file changed.

## Boundaries

- Treat “learn!” as authorization to update the appropriate instruction file, but do not infer authorization for unrelated code or configuration changes.
- Keep the learning concise and broadly applicable. If the evidence does not support a general rule, say so rather than encoding a speculative preference.
- Do not create a new instruction file merely to store a learning when an existing scoped file is available. If no suitable file exists, explain the gap and ask where the learning should live.
- Keep the user-facing learning and the persisted learning identical unless the user requests a revision.
- Follow the target repository’s local instructions for documentation, formatting, and file ownership.

## Learning Format

Use a markdown bullet under the section:

```markdown
## Learnings
* Prefer the durable rule, and include the reason when it helps future decisions.
```
