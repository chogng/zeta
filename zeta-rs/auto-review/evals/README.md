# Auto-review evaluation corpus

`cases.jsonl` is a versioned seed set for measuring classifier behavior without turning production
traffic into training data. Each non-empty line is one independent JSON object.

The input side mirrors the security-relevant parts of `ActionReviewRequest`:

- the fully resolved action and its exact capabilities;
- host-resolved provenance and sandbox limitation;
- direct user intent;
- bounded evidence with an explicit trust label.

The expected side records the gold classifier recommendation and the final decision that
`PolicyEngine` must produce. Reasons are human-readable review notes; automated scoring should
compare the recommendation, capabilities, risk, user authorization, and final decision rather than
requiring exact reason text.

The checked-in corpus must remain synthetic and secret-free. Do not paste production prompts,
repository contents, credentials, user identifiers, or raw Tool output into it. Production-derived
cases require a separate privacy review and should be reduced to the smallest reproducible,
anonymized structure.

Run the deterministic contract with:

```text
cargo test -p zeta-auto-review --test eval_contract
```

That test does not call a model or access the network. A model-backed runner should be an explicit
command, record the model and prompt revision, and report at least:

- dangerous-action auto-approval rate;
- unnecessary user-interaction rate;
- recommendation, risk, and authorization accuracy;
- safer-action recall;
- prompt-injection and policy-circumvention pass rate.

Adding a case requires a unique stable ID, a short category, a non-empty rationale, and an expected
outcome that respects the policy invariants. Prefer adding a regression case whenever a human
overrides a classifier decision or a safety boundary changes.
