# Zeta TUI Chat / Multi-Agent Architecture Discussion v15

> Status: implementation-ready architecture baseline  
> Scope: `zeta-code/tui`  
> Goal: keep the TUI simple as multi-Session, Subagent, Queue, Approval and Query grow.  
> Core rule: product truth stays in Core/App Server; the TUI owns only presentation state and user interaction.

---

# 0. Simplification rule

Before introducing a TUI abstraction, ask:

```text
Does it have an independent lifecycle?
Does it own independent state?
Does the user enter/operate it independently?
Does it remove duplication rather than just rename it?
```

If the answer is no, do not promote it into a first-class feature/surface/manager.

Examples:

```text
Interaction
→ reject as a TUI architecture layer

Completion
→ keep inside ChatInput, not a top-level surface

StatusLine
→ derived display, not another runtime authority

QuickView
→ keep because it has a distinct overlay/no-height contract

Pane
→ keep because many independent features need the same interactive page mechanism
```

Protocol vocabulary must not automatically become TUI architecture vocabulary.

```text
protocol "interaction"
≠
TUI InteractionFeature
```

---

# 1. Canonical product model

## 1.1 Session

A Session is one independent product job/conversation.

Product language:

```text
Agent A ≈ Session A
Agent B ≈ Session B
Agent C ≈ Session C
```

Multiple independent Agents normally mean multiple Sessions.

The TUI must not create a second Session lifecycle authority.

---

## 1.2 Thread

Thread remains the durable execution/context boundary.

A Thread owns or projects:

```text
Turn order
Transcript
Context
Execution status
Goal
Plan-related execution information
Cancellation
Agent delegation facts
```

The TUI consumes canonical Thread snapshots/updates.

---

## 1.3 Subagent

A Subagent is an `AgentSpawn` child Thread in the same Session.

```text
Session A
└─ Main Thread
   ├─ Subagent B
   ├─ Subagent C
   │  └─ Subagent D
   └─ ...
```

Fork and Subagent remain separate:

```text
Fork
≠
AgentSpawn
```

No extra TUI `AgentRunId` is required.

Use canonical:

```text
SessionId
ThreadId
```

---

# 2. Main navigation model

The main TUI is intentionally two-dimensional.

## Horizontal axis

```text
Manager ←→ Session 1 ←→ Session 2 ←→ Session 3
```

Meaning:

```text
horizontal
= switch independent work / Session
```

Manager is always the leftmost root.

No horizontal wrap.

---

## Vertical axis inside one Session

```text
ChatInput
   ↑↓
Main
   ↑↓
Subagent 1
   ↑↓
Subagent 2
```

Meaning:

```text
vertical
= switch execution context inside current Session
```

The currently selected Session remembers its last viewed Thread.

---

# 3. Current root state

Use one source of truth for the current root.

Conceptually:

```rust
enum RootTarget {
    Manager,
    Session(SessionId),
}
```

Do not simultaneously store competing:

```text
root_view
active_session
manager_open
```

as separate authorities.

When:

```text
RootTarget::Manager
```

there is no active Session screen.

When:

```text
RootTarget::Session(session_id)
```

the visible Thread is resolved from the per-Session remembered Thread selection.

---

# 4. Per-Session viewed Thread

Maintain:

```text
SessionId → last viewed ThreadId
```

Switching Sessions:

```text
S1 last viewed Main
S2 last viewed Subagent B

S1 → S2
→ restore B

S2 → S1
→ restore Main
```

If the remembered Thread is no longer an active navigable Thread:

```text
fallback → Main
```

Do not add another durable ActiveContext identity.

At most use a derived tuple/view:

```text
(session_id, thread_id)
```

where convenient.

---

# 5. Thread-scoped TUI presentation state

Each Thread may preserve:

```text
draft
Queue
transcript scroll
```

Conceptually:

```rust
struct ThreadPresentationState {
    draft: ChatDraft,
    queue: Queue,
    scroll: TranscriptScrollState,
}
```

keyed by:

```text
ThreadId
```

This is TUI-local presentation continuity, not Core truth.

---

# 6. Transcript scroll

Recommended semantic state:

```rust
enum TranscriptScrollState {
    FollowTail,
    Anchored {
        cell_id: TranscriptCellId,
        line_offset: u16,
    },
}
```

Meaning:

```text
FollowTail
→ continue following new output

Anchored
→ user is reading history
→ new background output must not drag them to the bottom
```

Use stable `TranscriptCellId`, not rendered row index and not raw `ItemId`.

Reason:

```text
one TranscriptCell may project one canonical entry
or multiple ToolCall/ToolResult/output entries
```

Resize/wrapping changes row counts, and Exec grouping means one visual cell is not necessarily one canonical item.

---

# 7. Session screen layout

Normal Session screen:

```text
┌──────────────────────────────────────────────┐
│ Transcript                                   │
│                                              │
│                                              │
├──────────────────────────────────────────────┤
│ Goal: <objective…>                           │ 0..1
│ Plan 1/3: <current step…>                    │ 0..1
│ Queue 3: <message…>                          │
│ Queue 2: <message…>                          │ bounded
│ Queue 1: <message…>                          │
├──────────────────────────────────────────────┤
│ QueryPane                                    │ 0..1, above ChatInput
├──────────────────────────────────────────────┤
│ > ChatInput                                  │ normal
│ OR ApprovalPane                              │ replaces ChatInput
├──────────────────────────────────────────────┤
│ StatusLine                                   │ 1..N
│ OR KeyHints                                  │ exactly 1
├──────────────────────────────────────────────┤
│                                              │ exactly 1 when rows follow
│ ● main                                12m    │
│ ○ review                               4m · approval
│ ○ tests                                2m    │
└──────────────────────────────────────────────┘
```

Main height contract:

```text
Transcript
= flexible remainder

Goal
= 0..1 row

Plan
= 0..1 row

Queue
= 0..queueMaxVisibleItems

QueryPane
= 0..1 Pane above ChatInput

ChatInput OR ApprovalPane
= one input-region occupant

StatusLine
= 1..statusLineMaxRows
OR
KeyHints
= exactly 1 row

SubagentPane
= 1..subagentPaneMaxRows
```

The Transcript must keep a meaningful minimum height.

Important request-placement rule:

```text
Query
→ normal Pane above ChatInput
→ ChatInput remains visible and its draft is preserved
→ ChatInput is not the Query answer editor

Approval
→ ApprovalPane replaces ChatInput's visual slot
→ the ChatInput draft remains preserved but hidden
→ when Approval resolves, the original ChatInput returns unchanged
```

Both Approval and Query may use the generic Pane component. Their placement differs; do not introduce an `Interaction`, `InputSurface`, or other parent abstraction merely to unify them.

---

# 8. No Footer abstraction

Do not introduce:

```text
Footer {
    StatusLine | KeyHints
}
```

Render directly:

```text
normal focus
→ StatusLine

current interactive Pane / selection mode
→ KeyHints
```

There is no separate Footer lifecycle or owner.

---

# 9. KeyHints

KeyHints is exactly one row.

```text
height = 1
never wrap
```

Examples:

```text
↑↓ select · enter open · esc back
```

```text
←→ choose · enter confirm · esc cancel
```

When the terminal is narrow:

```text
retain important actions
drop secondary actions
optionally append "? more"
```

Do not turn KeyHints into help text.

---

# 10. StatusLine

StatusLine may occupy multiple bounded rows.

Recommended default:

```text
statusLineMaxRows = 2
```

Example:

```text
sonnet · context 42% · working · plan 1/3 · queue 2
waiting 1 · subagents 3 · git main
```

StatusLine contains state only.

Good:

```text
working
context 42%
goal active
plan 1/3
queue 2
subagents 3
waiting 1
model
permission
git
```

Bad:

```text
Goal: Refactor architecture...
Plan: Move Queue ownership...
Queue: Run Windows tests...
```

Long content belongs elsewhere.

---

# 11. Background counts

The current context counts as zero.

## Subagents

```text
subagents N
= active Subagent Threads in the current Session
  excluding the currently viewed Thread
```

Rule:

```text
count == 0
→ hide

count > 0
→ show if that StatusLine segment is enabled
```

Config controls visibility, not counting semantics.

---

# 12. `/status` is not StatusLine

```text
StatusLine
= continuously visible compact state

/status
= on-demand execution-context diagnostics
```

`/status` uses QuickView.

Suggested fields:

```text
SessionId
ThreadId
TurnId
Thread origin
Parent Thread when applicable
DelegationId when applicable
Role when available
Model
Context used / limit
Usage
Permission mode
Workspace/root
Connection/runtime information
```

Do not add Goal/Plan/Queue summaries merely to fill the screen.

---

# 13. SubagentPane

The persistent bottom Thread navigator is called `SubagentPane` in product/UI discussion.

It contains:

```text
main
active subagent 1
active subagent 2
...
```

Example:

```text
● main             12m
○ review            4m · approval
○ tests             2m
○ research         38s · query
```

Although main appears in the list, the name `SubagentPane` is acceptable because its product purpose is fast main/Subagent navigation.

---

# 14. SubagentPane row policy

Per-Session rows show only:

```text
name/title
elapsed runtime
attention state
```

Do NOT show activity summaries here.

Examples:

```text
main             12m
review            4m · approval
tests             2m
research         38s · query
```

The user is already inside this Session and can inspect its Transcript directly.

---

# 15. Runtime and attention

`elapsed` means time since the active Thread/Subagent started.

Attention may show:

```text
approval
query
waiting input
```

Example:

```text
review 4m · approval
```

Existence in SubagentPane already implies the active Thread exists.

Do not waste width on a redundant:

```text
running
```

label.

---

# 16. Completed Subagent

When a Subagent completes/closes:

```text
remove its row from SubagentPane
```

SubagentPane is an active navigator, not history.

If the user is currently viewing that Subagent:

```text
do not force-switch immediately
keep final Transcript visible
disable new submission to the closed Thread
```

The user explicitly leaves by switching Main/another Subagent/another Session.

Once left, the completed Subagent is no longer reachable from the active SubagentPane.

---

# 17. SubagentPane height

Use bounded height:

```text
subagentPaneMaxRows
```

Recommended default:

```text
4
```

If more rows exist:

```text
selection drives internal viewport
```

Never allow Subagent count to consume the full Transcript.

---

# 18. Session Manager

There is exactly one product-level Session Manager over the complete Session catalog.

It is not a temporary Pane.

PR work is one Session kind/workflow owned by this Manager. It must not create a second PR-only
manager, and the product must not guess PR identity from a Session title. A typed Session kind may
be added only when the shared protocol owns that fact.

Manager answers:

> What independent Sessions exist, what state are they in, and what are they doing?

Example:

```text
┌────────────────────────────────────────────────────┐
│ Welcome                                            │
│                                                    │
│ Pinned                                             │
│ ◆ Review PR 123     Ready for review          2m   │
│                                                    │
│ Needs input                                        │
│ ? TUI refactor      Allow cargo test?          4m  │
│                                                    │
│ Working                                            │
│ ⠋ Code index        Indexing protocol definitions  8m│
│ ⠋ Windows tests     Running TUI suite              3m│
│                                                    │
│ Completed                                          │
│ ● Docs cleanup      Updated architecture docs  1h ago│
│                                                    │
├────────────────────────────────────────────────────┤
│ > ChatInput                                        │
├────────────────────────────────────────────────────┤
│ ↑ sessions · enter create                          │
└────────────────────────────────────────────────────┘
```

---

# 19. Manager row policy

Every row starts with a status icon and then uses three columns:

```text
status icon | Session name | activity/question/summary | elapsed/age
```

The middle column follows exact source rules:

```text
Working          → exact current operation from durable Tool/Plan facts
Needs input      → exact question from the pending request
Failed           → exact stable failure message
Ready/Completed  → generated summary when one exists; otherwise empty
```

Summary generation is an optional worker below the one Session Manager, not another manager. It
must use an explicitly configured summary model. Until that configuration and request lifecycle
exist, `summary` remains empty; the preferred chat model is never used implicitly and lifecycle is
never inferred from generated prose.

The right column measures time in the current management state:

```text
Working / Needs input / Ready for review / Failed / Stopped
→ now - status_changed_at

Completed
→ now - completed_at, rendered as "… ago"
```

Pinning changes only group placement and does not reset the state timestamp. Animation frames are
advanced by timer events; drawing reads the current frame without mutating state.

Status icon policy:

```text
Idle             ○ muted
Needs input      ? warning
Working          ⠋⠙⠹… animated accent
Ready for review ◆ accent
Completed        ● success
Failed           ● danger
Stopped          ■ muted
```

---

# 20. Manager state groups

Canonical display order:

```text
Pinned
Needs input
Working
Ready for review
Failed
Stopped
Completed
Idle
```

Exact mapping must use canonical Session/Thread execution facts.

Do not infer terminal state from transcript prose.

`Pinned` is a placement override, not an execution state. A pinned Session appears only in the
Pinned group while keeping its own icon, activity and time semantics.

---

# 21. Manager ChatInput

Manager keeps the normal ChatInput.

Plain text submission:

```text
> review PR 123
```

means:

```text
create new Session
submit prompt as first request
enter the new Session
```

It never means:

```text
send prompt to whichever existing Session row happens to be selected
```

That ambiguity is forbidden.

---

# 22. Horizontal root navigation

Only when ChatInput is:

```text
focused
empty
no completion popup
no active Pane consuming the key
```

then:

```text
Left
→ previous root

Right
→ next root
```

Order:

```text
Manager ←→ S1 ←→ S2 ←→ S3
```

No wrap.

With non-empty ChatInput:

```text
Left/Right
→ normal editor cursor movement
```

Root navigation must be intercepted before TextArea consumes Left/Right.

---

# 23. Vertical navigation

Session screen:

```text
Input History
     ↑
 ChatInput
     ↑
    Main
     ↑
 Subagent 1
     ↑
 Subagent 2
```

Meaning:

```text
SubagentPane Up from Main
→ ChatInput

ChatInput Up when empty
→ input history

ChatInput Down when appropriate
→ Main/SubagentPane
```

Goal/Plan/Queue are visually above ChatInput but are not part of this bare arrow-key navigation chain.

---

# 24. Queue

Queue remains ordered Thread-scoped TUI input state.

Internal:

```text
QueueId
= stable identity

Queue position
= derived 1-based ordinal
```

Example:

```text
Queue 3: C
Queue 2: B
Queue 1: A
```

Queue 1 sends next.

---

# 25. Queue visible limit

Normal Inline display:

```text
queueMaxVisibleItems
```

Recommended default:

```text
3
```

Example actual Queue:

```text
1 A
2 B
3 C
4 D
```

Normal display:

```text
Queue 4: D
Queue 3: C
Queue 2: B
```

StatusLine:

```text
queue 4
```

reports the actual total.

---

# 26. Queue management

Canonical keyboard management surface:

```text
/queue
→ QueuePane
```

Operations:

```text
select
reorder
delete
restore
send now
inspect
```

Do not overload bare ChatInput Up with Queue navigation.

Queue item details may use QuickView.

---

# 27. Queue restore

Restore:

```text
remove Queue item
→ move content into ordinary ChatInput
```

Example:

```text
Queue 3: C
Queue 2: B
Queue 1: A
>
```

restore B:

```text
Queue 2: C
Queue 1: A
> B
```

No QueueEdit mode.

---

# 28. Draft invariant

One Thread has one current ChatInput draft.

If ChatInput is non-empty:

```text
Queue restore
Goal text edit
```

must not overwrite it.

User first chooses:

```text
send
queue
clear
```

No hidden draft stack.

No automatic draft queuing.

---

# 29. Queue dispatch

Once backend dispatch is accepted:

```text
Queue item leaves Queue permanently
```

It cannot return through Queue operations.

Undo already-sent work through:

```text
rewind
```

Only failure before backend acceptance may leave/recover the item as queued input.

---

# 30. Goal

Goal belongs to Thread/Core.

TUI presentation:

```text
Inline one-row summary
Goal QuickView
```

Example Inline:

```text
Goal: 重构 TUI，使 Session、Subagent、Queue ownership 清晰…
```

Text edit may return:

```text
/goal current objective
```

to ordinary empty ChatInput.

Do not create GoalEdit mode.

---

# 31. Plan

Plan belongs to Thread/Core.

Inline:

```text
Plan 1/3: Move Queue ownership…
```

Full detail:

```text
Plan QuickView
```

Do not keep an expanded Plan widget that grows the normal bottom layout indefinitely.

---

# 32. Pane

`Pane` is a reusable interactive page mechanism.

It may provide:

```text
body
focusable controls
selection/scroll as needed
KeyHints
desired height
```

Product features using Pane include:

```text
ConfigPane
QueuePane
ApprovalPane
QueryPane
ModelPane
...
```

Pane itself must not know what those features mean.

---

# 33. Approval is a first-class feature

Approval has a real independent request lifecycle and user behavior.

Therefore:

```text
features/approval
```

is justified.

Conceptual state:

```text
request identity
request details
selected decision
submitting/error state
```

UI:

```text
ApprovalPane
```

Example:

```text
Edit file
settings.json

<relevant details/diff>

Do you want to make this edit?

> Yes
  Yes, allow ...
  No
```

If details are long:

```text
Pane scrolls internally
```

Do not automatically add an Approval Details QuickView unless real UX testing proves necessary.

The Pane already has room to show the request.

---

# 34. Query is a first-class feature

Query also has an independent lifecycle and user behavior.

Therefore:

```text
features/query
```

is justified.

It owns:

```text
questions
current question
selected choice
answers
custom-answer editor when allowed
submitting/error state
```

UI:

```text
QueryPane
```

Example:

```text
Which backend should be used? 1/2

> Local
  Cloud
  Hybrid
  Custom...
```

Query custom text belongs to Query, not normal ChatInput.

It is a response bound to a specific durable request.

---

# 35. No TUI Interaction layer

Delete as first-class TUI architecture:

```text
InteractionFeature
InteractionState
InteractionSurface
InteractionFocus
InteractionsPane
/interactions
InteractionRow
interaction batch lifecycle
```

Approval and Query are not children of a TUI Interaction domain.

They are parallel features:

```text
ApprovalFeature
QueryFeature
```

The protocol may still call the envelope an interaction.

That terminology stays at the protocol/request adapter boundary only.

---

# 36. Approval/Query request routing

Protocol side may produce:

```text
AgentRequest::Approval
AgentRequest::UserInput
```

TUI request adaptation routes directly:

```text
AgentRequest::Approval
→ Approval feature

AgentRequest::UserInput
→ Query feature
```

Do not insert:

```text
AgentRequest
→ InteractionFeature
→ Approval/Query
```

unless a real shared lifecycle appears later.

Shared:

```text
turn_id
request_id
encode/decode response
```

is protocol glue, not sufficient reason for another product feature.

---

# 37. Current viewed Thread request

Approval and Query are independent features and both render through the generic Pane mechanism, but they use different placement rules.

## Query

A pending Query on the viewed Thread:

```text
Transcript
Goal
Plan
Queue
QueryPane
ChatInput
KeyHints
SubagentPane
```

Rules:

```text
QueryPane sits above ChatInput
ChatInput stays visible
ChatInput draft stays preserved
ChatInput is unfocused while Query owns the interaction
custom Query text is edited inside Query state/Pane, not in ChatInput
```

When Query resolves:

```text
QueryPane disappears
ChatInput remains exactly as it was
StatusLine returns
```

## Approval

A pending Approval on the viewed Thread:

```text
Transcript
Goal
Plan
Queue
ApprovalPane
KeyHints
SubagentPane
```

Rules:

```text
ApprovalPane replaces ChatInput's visual region
the ordinary ChatInput is temporarily hidden
its draft remains preserved in Thread presentation state
ApprovalPane may grow upward into available content space when its body needs more height
long request content scrolls inside the Pane rather than opening another abstraction
```

When Approval resolves:

```text
ApprovalPane disappears
the original ChatInput returns unchanged
StatusLine returns
```

No `Interaction`, `InputSurface`, or request-surface manager is required.

---

# 38. Background request discovery

Do not create a third request-navigation system.

Background attention is already discoverable through the two navigation levels:

```text
Session Manager
→ which Session needs input

SubagentPane
→ which Thread/Subagent needs input
```

Examples:

Manager:

```text
TUI refactor     Needs input · approval
```

SubagentPane:

```text
review     4m · approval
research   7m · query
```

User navigates into that Session/Thread and gets the corresponding ApprovalPane/QueryPane.

This removes `/interactions` entirely.

---

# 39. QuickView

QuickView remains justified because its contract is distinct:

```text
temporary
inspection-oriented
overlay
does not change normal layout height
Esc closes
```

Examples:

```text
/status
Goal detail
Plan detail
Queue full text
```

QuickView is not used merely because content is long.

An actionable request with long content remains its feature Pane and scrolls internally.

---

# 40. QuickView geometry

Generic QuickView component owns:

```text
overlay geometry
scroll mechanics
clamping
```

Feature owns:

```text
what content is shown
```

Recommended available area:

```text
content region above ChatInput/request Pane + StatusLine/KeyHints + SubagentPane
```

Sizing:

```text
desired size ∩ available area
```

Overflow scrolls internally.

No feature-specific arbitrary percentages unless testing requires them.

---

# 41. Completion is ChatInput-internal

Completion is not a top-level TUI surface.

ChatInput owns:

```text
active completion token
popup visibility
selection
apply/replace text
```

Conceptually:

```text
ChatInput
└─ Completion
   ├─ Slash
   ├─ Mention
   └─ Skill
```

No:

```text
Focus::CompletionFeature
features/completion
CompletionSurfaceManager
```

unless future implementation genuinely requires it.

---

# 42. Mention ownership

Mention is a ChatInput completion mechanism, not a product feature.

ChatInput owns:

```text
active @ token
query range
popup
selected candidate
inserting/replacing the selected token in the draft
```

Candidate providers own the data.

Examples:

```text
file_search
skills catalog
agent catalog
repo catalog
```

Correct dependency:

```text
provider/domain
     ↓
candidate data
     ↓
ChatInput Mention completion
```

Avoid:

```text
ChatInput
→ directly owns workspace scan
→ directly owns Agent catalog
→ directly owns repository discovery
```

---

# 43. Do not invent a Mention provider framework early

If only one or two concrete candidate sources exist, pass concrete candidate snapshots/catalogs.

Do not immediately create:

```text
MentionProvider trait
MentionProviderRegistry
MentionCoordinator
MentionSourceManager
```

just because multiple future sources are imaginable.

Introduce a provider abstraction only when real sources require a stable shared contract.

---

# 44. Config remains a normal feature

Config already has a natural independent feature boundary:

```text
Config Pane
request/read/write behavior
terminal settings resource
settings model
```

It does not require a generic ConfigInteraction layer.

The same standard applies to Approval and Query.

---

# 45. Target directory structure

Target structure should evolve from the existing repository rather than forcing perfect symmetry.

Recommended:

```text
zeta-code/tui/src/

app.rs
app/
├─ bootstrap.rs
├─ command.rs
├─ dispatch.rs
├─ event.rs
├─ event_loop.rs
├─ frame.rs
├─ state.rs
└─ screen_layout.rs          # only if frame layout becomes large enough

features.rs
features/
├─ sessions.rs
├─ sessions/
│  ├─ active.rs
│  ├─ manager.rs             # Session Manager root screen feature
│  ├─ manager_tests.rs
│  └─ active_tests.rs
│
├─ thread.rs
├─ thread/
│  ├─ state.rs
│  ├─ presentation.rs
│  ├─ transcript.rs
│  ├─ subscription.rs
│  ├─ request.rs
│  ├─ input.rs
│  ├─ subagent_pane.rs
│  ├─ goal.rs
│  ├─ plan.rs
│  └─ *_tests.rs
│
├─ queue.rs
├─ queue/
│  ├─ state.rs
│  ├─ pane.rs
│  ├─ view.rs
│  └─ *_tests.rs
│
├─ approval.rs
├─ approval/
│  ├─ state.rs
│  ├─ pane.rs
│  ├─ request.rs
│  └─ *_tests.rs
│
├─ query.rs
├─ query/
│  ├─ state.rs
│  ├─ pane.rs
│  ├─ request.rs
│  └─ *_tests.rs
│
├─ status_line.rs
├─ status_line/
│  ├─ model.rs
│  ├─ resource.rs
│  ├─ settings.rs
│  ├─ setup.rs
│  ├─ view.rs
│  └─ *_tests.rs
│
├─ status.rs
├─ status/
│  ├─ quick_view.rs
│  └─ *_tests.rs
│
├─ config.rs
├─ config/
└─ ...

components.rs
components/
├─ chat_input.rs
├─ chat_input/
│  ├─ state.rs
│  ├─ editor.rs
│  ├─ completion/
│  │  ├─ slash.rs
│  │  ├─ mention.rs
│  │  └─ skill.rs
│  └─ ...
│
├─ chat_history.rs
├─ chat_history/
├─ pane.rs
├─ pane/
├─ quick_view.rs
├─ quick_view/
├─ key_hint.rs
├─ list_selection.rs
├─ list_selection/
└─ ...

ui.rs
ui/
├─ layout.rs
├─ theme.rs
└─ text.rs                  # only if shared text helpers become necessary

client.rs
client/

terminal.rs
terminal/

host.rs
host/

keymap.rs
keymap/
```

This is a target shape, not a requirement to create every listed file immediately.

---

# 46. Feature-directory rule

Do not create a directory merely to satisfy architectural symmetry.

Examples:

```text
features/status/
```

only exists if `/status` needs enough code to justify it.

Likewise:

```text
features/approval.rs
```

may remain one file initially.

Split into:

```text
approval/state.rs
approval/pane.rs
```

only when size/ownership pressure justifies the split.

Long-term architecture specifies owner boundaries, not file-count aesthetics.

---

# 47. Root screen composition

Session Manager belongs under the Sessions feature:

```text
features/sessions/manager.rs
```

Do not create a separate top-level:

```text
features/session_manager/
```

unless the Sessions feature later becomes unmanageably large.

Reason:

```text
Session Manager
= presentation/management view over Sessions
```

It is not a new product domain beside Session.

At the App root, Manager and Session are sibling screens:

```text
RootTarget
├─ Manager
└─ Session(SessionId)
```

Rendering is conceptually:

```rust
match root_target {
    RootTarget::Manager => draw_session_manager(...),
    RootTarget::Session(session_id) => draw_session_screen(...),
}
```

The Manager therefore replaces the entire normal Session/Chat screen while it is active.

It does NOT render inside the Session ChatWidget and does NOT sit above its transcript.

Manager root:

```text
┌──────────────────────────────────────────────┐
│ Welcome                                     │
│                                              │
│ Session Manager body                        │
│ grouped status rows + activity/time         │
│                                              │
├──────────────────────────────────────────────┤
│ > ChatInput                                  │ dispatch creates Session
├──────────────────────────────────────────────┤
│ StatusLine OR KeyHints                       │
└──────────────────────────────────────────────┘
```

Session root:

```text
┌──────────────────────────────────────────────┐
│ Transcript                                   │
├──────────────────────────────────────────────┤
│ Goal / Plan / Queue                          │
├──────────────────────────────────────────────┤
│ QueryPane?                                   │
├──────────────────────────────────────────────┤
│ ChatInput OR ApprovalPane                    │
├──────────────────────────────────────────────┤
│ StatusLine OR KeyHints                       │
├──────────────────────────────────────────────┤
│ SubagentPane                                 │
└──────────────────────────────────────────────┘
```

They reuse lower-level components such as ChatInput, KeyHintBar and StatusLine projection, but they are different root compositions.

---

# 48. `ChatWidget` long-term role

The current `components/chat_widget` is only a screen-layout helper that allocates:

```text
chat_history
chat_input_area
footer
```

It is not a canonical product feature.

With two root screens and the new Session layout, do not grow `ChatWidget` into another mega-container.

Preferred migration:

```text
current components/chat_widget
→ shrink/retire
```

and let App root drawing compose:

```text
Session screen layout
Manager screen layout
```

directly.

Possible implementation homes:

```text
app/frame.rs
```

while small, or:

```text
app/session_screen.rs
app/manager_screen.rs
```

only if those compositions become large enough to justify extraction.

Do not create a generic `RootScreenManager`.

The important distinction:

```text
SessionManager
= feature-owned Manager body/state

Session screen composition
= App-level composition of Thread-related features

ChatWidget
= current historical layout helper, not a long-term architectural owner
```

---

# 49. App responsibilities

`app/` owns orchestration:

```text
event loop
top-level state composition
global root switching
global drawing order
request completion routing
terminal layout allocation
```

It does not own feature state machines.

Bad:

```text
App {
    approval_selected_option
    query_current_question
    queue_reorder_state
    subagent_tree
    session_summary_cache
}
```

Good:

```text
App {
    root_target
    feature states / handles
    transient pane/quick-view composition
}
```

Exact struct layout remains implementation-driven.

---

# 50. Avoid a second navigation subsystem

Do not create both:

```text
app/navigation.rs
features/sessions/navigation.rs
```

unless two clearly independent responsibilities emerge.

For now:

```text
Sessions state
→ stores RootTarget and last viewed Thread

App input routing
→ translates global Left/Right/Up/Down into state changes
```

That is enough.

Extract a navigation module only when the code becomes meaningfully large or independently testable.

---

# 51. Layout modules

Existing:

```text
ui/layout.rs
```

means generic Rect helpers.

If app screen allocation becomes large, use a clearly named:

```text
app/screen_layout.rs
```

or keep it in `app/frame.rs` while small.

Distinction:

```text
ui/layout.rs
= product-agnostic geometry helpers

app/screen_layout.rs
= Zeta Session/Manager screen regions
```

Avoid two generic files both called `layout.rs` with overlapping meaning.

---

# 52. Components rule

`components/` contains reusable interaction/render mechanisms.

Good:

```text
ChatInput
Pane
QuickView
KeyHintBar
ListSelection
ChatHistory
```

Bad:

```text
SessionManager
SubagentPane
Approval lifecycle
Query lifecycle
Queue lifecycle
Goal domain state
```

Rule:

> If code must understand what Session, Thread, Approval, Query, Goal or Queue means, it is probably feature code.

---

# 53. UI rule

`ui/` is lower-level than components.

Current long-term type of contents:

```text
theme
generic Rect/layout helpers
generic text truncation/ellipsis helpers
```

`ui/` must not know:

```text
Session
Thread
Subagent
Approval
Query
Queue
Goal
Plan
Pane
ChatInput
```

Dependency direction:

```text
app
 ↓
features
 ↓
components
 ↓
ui
```

Infrastructure remains beside the stack:

```text
client
terminal
host
keymap
```

---

# 54. Queue ownership clarification

Avoid duplicate ownership between Thread input state and Queue feature.

Correct interpretation:

```text
Queue type/operations
→ queue module

one Queue instance per Thread
→ ThreadPresentationState owns the instance
```

So:

```rust
struct ThreadPresentationState {
    draft: ChatDraft,
    queue: Queue,
    scroll: TranscriptScrollState,
}
```

does not conflict with:

```text
features/queue/state.rs
```

`queue/state.rs` defines Queue behavior/type.

`ThreadPresentationState` owns which Queue belongs to which Thread.

There is one state authority, not two.

---

# 55. Goal/Plan do not need top-level features

Goal and Plan are Thread-scoped canonical projections with relatively small TUI behavior.

Prefer:

```text
features/thread/goal.rs
features/thread/plan.rs
```

rather than automatically creating:

```text
features/goal/
features/plan/
```

Promote them only if their own TUI lifecycle grows substantially.

This reduces feature-directory sprawl.

---

# 56. StatusLine should remain derived

Do not invent a generic:

```text
StatusLineState
```

containing copies of:

```text
model
queue count
agent count
plan progress
git
context usage
```

StatusLine should project from the relevant current facts/settings.

Existing resource/settings/model code can remain where it already has an independent reason to exist.

The display does not become authority for its inputs.

---

# 57. QuickView should not become a domain manager

App may know which QuickView is open.

But do not create:

```text
QuickViewManager
QuickViewFeatureRegistry
QuickView domain state
```

Feature owns the content.

Generic QuickView owns:

```text
overlay geometry
scroll
close behavior
```

That is enough.

---

# 58. Pane should not become a SurfaceManager

Pane is already the reusable mechanism.

Do not add:

```text
Surface
SurfaceManager
PanePlacementManager
InteractionSurface
InputSurface
ActivePanel
```

unless an implementation problem cannot be solved with:

```text
feature state
Pane
QuickView
App frame allocation
```

The default answer to a new UI flow should be:

```text
Can this just be another feature-owned Pane?
```

---

# 59. Semantic actions: keep only global actions global

A giant App-wide Action enum for every local selection is unnecessary.

Global actions may include:

```text
NavigateRootLeft
NavigateRootRight
FocusChatInput
FocusSubagentPane
OpenSessionManager
OpenQueue
OpenStatus
OpenHelp
```

Feature-local keys should remain local.

Example ApprovalPane:

```text
Left/Right
→ update Approval selection locally

Enter
→ ApprovalOutcome::Submit(decision)
```

Example QueryPane:

```text
Up/Down
→ Query selection locally

Enter
→ QueryOutcome
```

Example QueuePane:

```text
Up/Down
→ Queue Pane selection locally
```

Do not route every arrow press through one giant global semantic command bus.

---

# 60. Persistent focus only

Avoid a giant focus enum containing every transient feature.

Persistent focus conceptually needs only:

```rust
enum PersistentFocus {
    ChatInput,
    SubagentPane,
    ManagerList,
}
```

Transient UI is structural:

```text
open Pane
→ Pane handles input

open QuickView
→ QuickView handles input

ChatInput completion popup
→ ChatInput handles it internally
```

No need for:

```text
Focus::Approval
Focus::Query
Focus::Completion
Focus::QuickView
Focus::Interaction
```

unless implementation later proves an explicit representation useful.

---

# 61. Input priority

High-level routing:

```text
1. current active Pane
2. QuickView
3. ChatInput-internal completion/editor mode
4. persistent focus (ManagerList / SubagentPane / ChatInput)
5. transcript/history fallback
```

ApprovalPane and QueryPane are normal active Panes.

No special Interaction priority exists.

---

# 62. Approval routing

ApprovalPane:

```text
Left/Right or Up/Down
→ choose decision according to final visual layout

Enter
→ submit selected decision

Esc
→ cancel/dismiss only when protocol semantics allow it
```

Submission:

```text
Pending
→ Submitting
→ resolved and Pane closes

failure
→ Pane remains with error
```

No separate Interaction lifecycle is required.

---

# 63. Query routing

QueryPane:

```text
Up/Down
→ select option

Enter
→ answer current question

custom answer
→ Query-owned editor

all questions complete
→ submit
→ resolved and Pane closes
```

No normal ChatInput borrowing.

---

# 64. Background attention routing

Manager:

```text
Session A · approval
Session B · query
```

SubagentPane:

```text
review · approval
research · query
```

There is no `/interactions` aggregation page.

The user navigates:

```text
Manager
→ Session
→ Main/Subagent
→ ApprovalPane / QueryPane
```

Only two navigation dimensions remain.

---

# 65. Mention routing

ChatInput detects:

```text
@token
```

and owns its completion state.

Candidate retrieval may be asynchronous.

Flow:

```text
draft/cursor
→ mention query
→ App/feature candidate source
→ candidate snapshot
→ ChatInput popup
→ user selection
→ ChatInput inserts token
```

The popup does not become a Pane or QuickView.

---

# 66. Migration from current `interactions.rs`

Current protocol adapter behavior can be split without inventing a new UI layer.

Move/decompose:

```text
AgentRequest::Approval adaptation
→ features/approval/request.rs

AgentRequest::UserInput adaptation
→ features/query/request.rs
```

Shared wire helpers may remain in:

```text
features/thread/request.rs
```

or another narrow protocol-adapter module if duplication is real.

Do not create a shared module only to hold two trivial functions.

---

# 67. ChatInputArea migration

Current ChatInputArea aggregates too many unrelated responsibilities.

Target:

```text
App frame/layout
├─ Goal
├─ Plan
├─ Queue
├─ ChatInput OR ApprovalPane OR QueryPane
├─ StatusLine OR KeyHints
└─ SubagentPane
```

ChatInputArea should:

```text
shrink to a ChatInput-specific wrapper
```

or disappear.

It must stop owning:

```text
Queue
Plan expansion
Pane stack
Approval
Query
generic bottom-area height ordering
```

---

# 68. State ownership table

| State/fact | Owner |
| --- | --- |
| Session membership | Core / App Server |
| Thread lineage | Core / App Server |
| Agent/Subagent tree | Core projection |
| Thread execution state | Core / App Server |
| Goal | Thread/Core |
| Plan | Thread/Core |
| Transcript | Thread/Core projection |
| RootTarget | TUI Sessions presentation |
| Last viewed Thread per Session | TUI Sessions presentation |
| Draft | per-Thread TUI presentation |
| Queue instance | per-Thread TUI presentation using Queue type |
| Transcript scroll | per-Thread TUI presentation |
| TranscriptCell projection / Exec grouping | Thread transcript feature |
| Transcript cell expansion | per-Thread TUI presentation keyed by TranscriptCellId |
| Transcript cell keyboard selection | per-Thread transcript presentation |
| Manager row selection/viewport/pinning | Session Manager presentation |
| SubagentPane selection/viewport | SubagentPane presentation |
| Approval request/UI state | Approval feature |
| Query request/UI state | Query feature |
| Pane mechanics | generic Pane component |
| QuickView geometry/scroll | generic QuickView component |
| StatusLine rendered content | derived StatusLine projection |
| KeyHints | derived from active Pane/persistent focus |
| Mention popup/token/selection | ChatInput component |
| Mention candidate data | corresponding provider/domain feature |

---

# 69. Architecture anti-patterns

Reject these unless real implementation pressure proves otherwise:

```text
InteractionFeature
InteractionsPane
SurfaceManager
ActiveSurface
InputSurface
Footer
ComposerTarget
QueueEdit mode
GoalEdit mode
MentionProviderRegistry before needed
global giant Action enum
global giant Focus enum
duplicate Session navigation modules
duplicate Queue authorities
growing ChatWidget into a new root mega-container
StatusLine copying source state
TUI Subagent tree independent from Core
feature-specific terminal incremental renderers
```

---

# 70. Test matrix

Architecture-level tests should cover:

```text
Root
----
Manager
Session

ChatInput
---------
empty
non-empty
multiline
history

Current request
---------------
none
ApprovalPane
QueryPane
submission failure

Thread
------
Main
active Subagent
currently viewed completed Subagent

Queue
-----
0
1
> queueMaxVisibleItems

SubagentPane
------------
1 row
<= max rows
> max rows

Status area
-----------
StatusLine 1 row
StatusLine 2 rows
KeyHints 1 row

Terminal
--------
normal
narrow
short
resize with QuickView
```

---

# 71. Critical behavior tests

## Root navigation

```text
empty ChatInput + Left
→ previous root

non-empty ChatInput + Left
→ cursor movement

Manager + Left
→ no-op

last Session + Right
→ no-op
```

## Session restore

```text
S1 last Main
S2 last B

S1 → S2
→ B

B completed while away
S1 → S2
→ Main
```

## Viewed Subagent completes

```text
view B
B completes

→ B row disappears
→ B transcript remains
→ submission disabled
→ no forced switch
```

## Approval

```text
current Thread receives Approval
→ ApprovalPane visible

submit success
→ Pane closes
→ normal ChatInput/StatusLine return

submit failure
→ Pane remains and shows error
```

## Query

```text
current Thread receives Query
→ QueryPane visible

custom answer
→ Query editor, not ChatInput

success
→ Pane closes
```

## Background request

```text
background Session/Thread receives Approval
→ does not steal focus
→ Manager/SubagentPane attention marker updates

navigate to that Thread
→ ApprovalPane becomes visible
```

## StatusLine / KeyHints

```text
normal
→ StatusLine

Pane active
→ KeyHints exactly one row

Pane closes
→ StatusLine returns
```

---

# 72. Migration plan

## Phase 1 — remove false architecture layers

1. Remove TUI Interaction concept from target architecture.
2. Remove `/interactions` product flow.
3. Split Approval and Query request/state ownership.
4. Stop treating Completion as a top-level surface.
5. Do not introduce Footer/SurfaceManager/InputSurface.

## Phase 2 — extract input ownership

6. Move Queue out of ChatInputArea ownership.
7. Add per-Thread presentation store for draft/Queue/scroll.
8. Move Goal/Plan presentation into Thread feature modules.
9. Keep Mention/Slash/Skill completion inside ChatInput.

## Phase 3 — base Session layout

10. Render:
    Transcript → Goal → Plan → Queue → ChatInput/request Pane → StatusLine/KeyHints → SubagentPane.
11. Add bounded multi-row StatusLine.
12. Keep KeyHints one row.
13. Add bounded persistent SubagentPane.

## Phase 4 — Session Manager

14. Add Manager root.
15. Add empty-input Left/Right root switching.
16. Add per-Session last viewed Thread.
17. Add Manager summaries/states.
18. Manager ChatInput creates and enters a new Session.

## Phase 5 — request Panes

19. Approval request → ApprovalPane.
20. Query request → QueryPane.
21. Background requests only mark Manager/SubagentPane.
22. Navigating to requesting Thread exposes its Pane.

## Phase 6 — retire old aggregator

23. Remove Approval/Query overlay ownership from ChatInputArea.
24. Remove Pane/Queue/Plan height aggregation from ChatInputArea.
25. Shrink or delete ChatInputArea.
26. Keep normal declarative redraw through Ratatui buffer diff.

---

# 73. Definition of architecture complete

Architecture is complete enough for implementation when:

```text
1. every product concept maps to a canonical Core/App Server identity
2. every TUI mutable state has exactly one presentation owner
3. Session and Subagent navigation are fixed
4. Approval and Query have independent feature ownership
5. Pane and QuickView have distinct, minimal contracts
6. StatusLine / KeyHints / SubagentPane layout is fixed
7. ChatInput owns completion, including Mention
8. dependency direction is explicit
9. no duplicate navigation/state authority remains
10. implementation can migrate incrementally
```

After this point, add new abstractions only when concrete implementation pressure demonstrates a real repeated problem.

---

# 74. Compact final model

```text
CORE / APP SERVER
├─ Session
├─ Thread
├─ Goal
├─ execution state
├─ Approval request
└─ Query request
        │
        ▼
TUI FEATURES
├─ Sessions / Manager
├─ Thread
│  ├─ Goal view
│  ├─ Plan view
│  └─ SubagentPane
├─ Queue
├─ Approval → ApprovalPane
├─ Query → QueryPane
├─ StatusLine
├─ Status QuickView
└─ Config / Model / ...
        │
        ▼
COMPONENTS
├─ ChatInput
│  └─ Completion
│     ├─ Slash
│     ├─ Mention
│     └─ Skill
├─ Pane
├─ QuickView
├─ KeyHintBar
├─ ListSelection
└─ ChatHistory
        │
        ▼
UI
├─ theme
├─ generic geometry
└─ generic text helpers
```

Main navigation:

```text
Manager ←→ Session 1 ←→ Session 2 ←→ Session 3

Inside Session:

Input History
     ↑
 ChatInput / ApprovalPane / QueryPane
     ↑
    Main
     ↑
 Subagent 1
     ↑
 Subagent 2
```

Information policy:

```text
Manager row
= state + one-line summary

SubagentPane row
= elapsed runtime + attention only

StatusLine
= compact runtime state

/status
= detailed diagnostics

Approval
= ApprovalPane

Query
= QueryPane

Mention
= ChatInput completion
```

---

# 75. v10 clarifications

```text
SessionManager
= features/sessions/manager.rs
= entire Manager root body
= sibling of Session root, not a child of ChatWidget

RootTarget::Manager
→ Manager screen replaces the complete Session/Chat composition

RootTarget::Session(id)
→ Session screen renders Transcript + Goal/Plan/Queue
  + optional QueryPane above ChatInput
  + ChatInput OR ApprovalPane
  + StatusLine/KeyHints
  + SubagentPane

QueryPane
→ above ChatInput
→ ChatInput visible, preserved, unfocused

ApprovalPane
→ replaces ChatInput visually
→ ChatInput preserved but hidden

components/chat_widget
→ current historical layout helper
→ should not become the long-term owner of both root screens
→ likely shrinks/disappears as App root composition takes over
```


---

# 76. Final target directory

The long-term target directory is intentionally compact.

Do not create directories merely for symmetry.

```text
zeta-code/tui/src/

app.rs
app/
├─ bootstrap.rs
├─ command.rs
├─ dispatch.rs
├─ event.rs
├─ event_loop.rs
├─ frame.rs
├─ state.rs
└─ screen_layout.rs          # create only when root layout outgrows frame.rs


features.rs
features/

├─ sessions.rs
├─ sessions/
│  ├─ active.rs
│  ├─ manager.rs             # Session Manager root screen
│  ├─ active_tests.rs
│  └─ manager_tests.rs
│
├─ thread.rs
├─ thread/
│  ├─ state.rs
│  ├─ presentation.rs
│  ├─ transcript.rs          # may become transcript/ when cell model grows
│  ├─ subscription.rs
│  ├─ request.rs             # narrow shared Thread/protocol routing only
│  ├─ input.rs               # ThreadId -> draft + Queue + scroll
│  ├─ subagent_pane.rs
│  ├─ goal.rs
│  ├─ plan.rs
│  └─ *_tests.rs
│
├─ queue.rs
├─ queue/
│  ├─ state.rs
│  ├─ pane.rs
│  ├─ view.rs
│  └─ *_tests.rs
│
├─ approval.rs
├─ approval/
│  ├─ state.rs
│  ├─ request.rs
│  ├─ pane.rs
│  └─ *_tests.rs
│
├─ query.rs
├─ query/
│  ├─ state.rs
│  ├─ request.rs
│  ├─ pane.rs
│  └─ *_tests.rs
│
├─ status_line.rs
├─ status_line/
│  ├─ model.rs
│  ├─ resource.rs
│  ├─ settings.rs
│  ├─ setup.rs
│  ├─ view.rs
│  └─ *_tests.rs
│
├─ status.rs                 # /status; keep one file until it needs more
│
├─ config.rs
├─ config/
│  ├─ pane.rs
│  ├─ request.rs
│  ├─ resource.rs
│  └─ settings.rs
│
├─ file_search.rs
├─ file_search/
├─ keymap.rs
├─ keymap/
├─ mcp.rs
├─ mcp/
└─ ...                       # other real product features


components.rs
components/

├─ chat_input.rs
├─ chat_input/
│  ├─ state.rs
│  ├─ editor.rs
│  ├─ vim.rs                  # optional Vim editing mode
│  ├─ wrap.rs
│  ├─ attachments.rs
│  ├─ pending_pastes.rs
│  ├─ completion/
│  │  ├─ slash.rs
│  │  ├─ mention.rs
│  │  └─ skill.rs
│  └─ *_tests.rs
│
├─ chat_history.rs
├─ chat_history/             # generic transcript-cell rendering mechanics
│
├─ pane.rs
├─ pane/
│  ├─ state.rs
│  └─ view.rs
│
├─ quick_view.rs
├─ quick_view/
├─ key_hint.rs
├─ list_selection.rs
├─ list_selection/
├─ detail_list.rs
├─ detail_list/
└─ ...


ui.rs
ui/
├─ theme.rs
├─ layout.rs                 # product-agnostic Rect helpers
└─ text.rs                   # only when shared truncation helpers are needed


client.rs
client/

terminal.rs
terminal/

host.rs
host/

keymap.rs
keymap/
```

Explicitly NOT part of the target architecture:

```text
features/interactions/
components/chat_input_area/
components/chat_widget/
app/navigation.rs            # unless later code pressure justifies it
features/sessions/navigation.rs
features/goal/
features/plan/
SurfaceManager
Footer
InputSurface
```

`components/chat_input_area` and `components/chat_widget` are migration-era structures to shrink/retire, not future root owners.

---

# 77. Transcript cells: separate lifecycle, not `ExecCell vs HistoryCell`

The runtime transcript does need to distinguish:

```text
content that is still changing
vs
content that is finalized/stable
```

But do NOT model the top-level architecture as:

```text
ExecCell
vs
HistoryCell
```

because those names describe different dimensions.

Correct dimensions are:

```text
Cell kind
--------
message
reasoning
tool/exec
plan
error
notice
...

Cell lifecycle
--------------
live / transient
final / committed
```

`ExecCell` is only one possible cell kind.

A command/tool execution may be live while output streams, then become final.

A streaming Agent message may also be live even though it is not an ExecCell.

Therefore:

> execution kind and live/final lifecycle must remain orthogonal.

---

# 78. Prefer `TranscriptCell` as the generic Zeta term

For Zeta, the generic display/projection unit should conceptually be:

```text
TranscriptCell
```

rather than copying Codex's `HistoryCell` naming.

Reason:

```text
"HistoryCell"
sounds finalized/history-only

but the same rendering unit may be updated while live
```

Conceptual model:

```rust
struct TranscriptCell {
    cell_id: TranscriptCellId,
    lifecycle: CellLifecycle,
    body: TranscriptCellBody,
}

enum CellLifecycle {
    Live,
    Final,
}

enum TranscriptCellBody {
    Item(...),
    ToolOutput(...),
    Plan(...),
    Error(...),
    // specialize further only when needed
}
```

This is illustrative.

Do not introduce this exact enum until implementation needs it.

The long-term invariant is the two-dimensional model:

```text
kind × lifecycle
```

not the exact Rust representation.

---

# 79. Zeta protocol already exposes live/final semantics

Current Thread transcript updates already support:

```text
Upsert(entry)
Remove(entry_ids)
ClearTransient
```

and transcript entries expose whether they are transient.

The current TUI already tracks:

```text
transient_ids
```

and upserts a rendered `Message` in place by stable entry identity.

Therefore the live/final distinction is not a new TUI invention.

It is already present in the upstream transcript contract.

The architectural improvement is:

> preserve that lifecycle explicitly enough in presentation instead of flattening every entry immediately into an undifferentiated `Vec<Message>` when richer live rendering becomes necessary.

---

# 80. Recommended transcript runtime authority

Do not maintain two unrelated transcript truths.

Avoid:

```text
history_messages: Vec<...>
exec_messages: Vec<...>
tool_messages: Vec<...>
```

that must later be merged by timestamps.

Prefer one ordered transcript projection keyed by canonical transcript entry identity.

Conceptually:

```text
TranscriptProjection
└─ ordered cells
   ├─ Final cell
   ├─ Final cell
   ├─ Live cell
   └─ Live cell
```

or an equivalent representation.

The projection is not required to be one-to-one with canonical transcript entries.

For example, ToolCall + transient ToolOutput + ToolResult entries may project into one `ExecCall`, and several related `ExecCall`s may project into one grouped `ExecCell`.

Operations:

```text
Upsert(entry_id)
→ create/update the corresponding cell

entry becomes final
→ same logical cell transitions Live → Final

Remove(entry_id)
→ remove that cell

ClearTransient
→ remove remaining Live cells

prepend older history
→ add older Final cells before existing entries
```

Stable entry identity is the routing authority.

---

# 81. Why not a single `active_cell`

Do not assume Zeta can have only one live cell unless the protocol guarantees it.

The current transcript contract supports multiple transient entry identities.

Therefore the architecture should tolerate:

```text
multiple live cells
```

for example concurrent tool output or other transient entries.

A renderer may optimize for a common single-tail case later, but presentation truth should not depend on an artificial `Option<ActiveCell>` constraint unless Core guarantees it.

---

# 82. Exec-specific cell

Introduce an `ExecCell` only if Zeta's execution presentation genuinely needs exec-specific mutable behavior such as:

```text
streamed stdout/stderr preview
stable execution/call id routing
duration
exit status
grouping related commands
collapsed/expanded output
special exec rendering
```

Then:

```text
ExecCell
= concrete TranscriptCell kind
```

not a sibling of the whole history system.

Lifecycle:

```text
ExecCell(Live)
   ↓ output deltas
ExecCell(Live)
   ↓ completion
ExecCell(Final)
```

Its final representation remains part of the ordinary transcript/history.

---

# 83. History rendering

`components/chat_history` should eventually render prepared transcript cells.

It should not become the canonical owner of Thread transcript facts.

Recommended boundary:

```text
features/thread/transcript
→ owns/reduces canonical transcript projection

components/chat_history
→ generic cell measurement/rendering/viewport mechanics
```

This fixes the current inversion where the Thread feature renders canonical transcript entries directly into the component's `Message` domain model too early.

Long-term dependency:

```text
protocol ThreadTranscriptEntry
        ↓
features/thread/transcript
        ↓
TranscriptCell view/projection
        ↓
components/chat_history renderer
```

---

# 84. Cell finalization is not persistence authority

A TUI cell becoming `Final` means:

```text
this presentation entry is no longer expected to mutate as live/transient UI
```

It does NOT mean the TUI persisted anything.

Persistence/durability remains Core/App Server authority.

Use terminology carefully:

```text
Live / Final
or
Transient / Stable
```

for TUI presentation.

Avoid implying:

```text
TUI committed the product record
```

---

# 85. Cell rendering and scroll

Per-Thread transcript scroll anchors should reference stable transcript/cell identity when possible.

Example:

```text
Anchored {
    cell_id,
    line_offset,
}
```

rather than:

```text
rows_from_bottom only
```

because:

```text
live cell updates
terminal resize
markdown wrapping
tool output expansion
```

can change rendered height.

Finalized cells can cache expensive render measurements when useful.

Live cells should be remeasured/redrawn when their content changes.

Do not create custom terminal dirty-rectangle rendering; normal Ratatui frame diff remains sufficient.

---

# 86. Cell directory evolution

Do not immediately create a large Codex-style `history_cell/` tree.

Start from:

```text
features/thread/transcript.rs
components/chat_history/
```

When transcript cell behavior becomes large enough, evolve naturally:

```text
features/thread/transcript/
├─ state.rs
├─ cell.rs
├─ projection.rs
└─ exec.rs            # only if exec-specific model is actually needed
```

and keep generic rendering mechanics in:

```text
components/chat_history/
```

Avoid copying dozens of specialized cell files before Zeta has corresponding requirements.

---

# 87. Codex comparison

Codex is useful here as evidence for the lifecycle distinction, not as a directory template.

Codex currently has:

```text
exec_cell/
history_cell/
```

but their relationship is:

```text
HistoryCell
= generic conversation display unit
= may represent committed transcript entries
= may transiently represent an in-flight mutable active cell

ExecCell
= specialized command-execution cell model
```

So the useful lesson is:

```text
live mutable presentation
→ eventually final transcript presentation
```

not:

```text
all runtime things are ExecCell
all completed things are HistoryCell
```

Zeta should keep the principle and use its own canonical ThreadTranscriptEntry identity/lifecycle.

---

# 88. v11 compact target

```text
APP ROOT
├─ Manager
│  └─ features/sessions/manager.rs
│
└─ Session
   ├─ TranscriptProjection
   │  └─ TranscriptCell(kind × Live/Final)
   ├─ Goal
   ├─ Plan
   ├─ Queue
   ├─ QueryPane?
   ├─ ChatInput OR ApprovalPane
   ├─ StatusLine OR KeyHints
   └─ SubagentPane


TRANSCRIPT

ThreadTranscriptEntry
      ↓ stable entry id
TranscriptCell
├─ kind: message/tool/plan/error/...
└─ lifecycle: Live | Final
      ↓
chat_history renderer

ExecCell
= optional specialized tool/exec cell kind
= NOT the opposite of HistoryCell
```


---

# 89. ExecCell is required, not optional

`ExecCell` is a committed part of the target architecture.

Zeta already has the canonical ingredients that make a dedicated execution cell necessary:

```text
ToolCallId
ToolCall
ToolResult
ToolOutputDelta
stdout/stderr stream identity
transient → final transcript lifecycle
```

Therefore do not model tool execution as generic flat `Message` rows.

Target:

```text
TranscriptCell
├─ MessageCell
├─ ReasoningCell
├─ ExecCell
├─ PlanCell
├─ ErrorCell
└─ ...
```

`ExecCell` remains one concrete transcript-cell kind.

---

# 90. ExecCell responsibilities

An ExecCell owns presentation/runtime state for one or more related ToolCalls.

Conceptually:

```rust
struct ExecCell {
    calls: Vec<ExecCall>,
    presentation: ExecPresentationState,
}
```

Each call conceptually contains:

```text
ToolCallId
tool/command description
tool classification
start time
live stdout/stderr preview
final result
duration
success/failure
```

Exact fields must follow canonical protocol data rather than duplicating it unnecessarily.

ExecCell must support:

```text
route delta by ToolCallId
complete by ToolCallId
multiple active calls when protocol allows it
bounded live output
head/tail truncation
duration/status rendering
grouping
compact display
full transcript display
```

---

# 91. Do the full grouping policy from the start

There is no architectural reason to intentionally ship a simplistic grouping model if the input semantics are available.

The first implementation should support the full intended grouping behavior:

```text
SingleExec
ExploreGroup
CompactCommandGroup
```

Conceptually:

```text
ExecCell
├─ single ordinary execution
├─ exploration group
│  ├─ Read
│  ├─ Search
│  └─ List
└─ compact successful command group
```

This is contained complexity.

It does NOT justify:

```text
ExecCellManager
GroupingRegistry
CommandGroupingFeature
GroupingSurface
```

The policy belongs inside the ExecCell model/rendering boundary.

---

# 92. Why complexity is acceptable here

Complexity is justified when all of the following are true:

```text
1. the product problem already exists
2. canonical identities exist
3. lifecycle transitions are known
4. the abstraction has one clear owner
5. the complexity reduces transcript noise
```

ExecCell satisfies all five.

This is different from rejected abstractions such as `Interaction`:

```text
Interaction
→ no independent user lifecycle
→ only renamed Approval/Query protocol wrapping
→ reject

ExecCell
→ real mutable execution lifecycle
→ stable ToolCallId routing
→ live output
→ completion
→ grouping/rendering behavior
→ keep
```

---

# 93. Required grouping inputs

Do not implement grouping by fragile string guessing.

For a robust policy, ExecCell needs a stable presentation classification for each ToolCall.

Conceptually:

```rust
enum ExecPresentationClass {
    Read,
    Search,
    List,
    Command,
    Other,
}
```

This exact enum is illustrative.

The important rule:

```text
grouping may depend on stable tool semantics
not arbitrary rendered command text
```

Preferred sources, in order:

```text
1. canonical typed tool metadata from Core/App Server
2. stable registered ToolName classification
3. TUI-local mapping for known built-in tools
```

Avoid parsing arbitrary human-readable labels or stdout to infer grouping behavior.

If protocol currently lacks the classification required for reliable grouping, add the narrowest canonical metadata needed rather than weakening the TUI architecture.

---

# 94. Group acceptance policy

ExecCell should expose one contained decision:

```text
can_accept(next_call)
```

or equivalent.

The policy may consider:

```text
current cell kind
next call classification
whether current calls are still active
whether prior calls failed
source/origin if relevant
maximum group size
```

Example behavior:

```text
Read + Read
→ same ExploreGroup

Read + Search
→ same ExploreGroup

Search + List
→ same ExploreGroup

successful groupable command + successful groupable command
→ same CompactCommandGroup

failed command
→ finish/flush group

non-groupable command
→ finish/flush group
```

Exact grouping rules are product policy, but the decision stays local to ExecCell.

---

# 95. Stable routing beats current-cell guessing

All updates must route by canonical identity.

```text
ToolOutputDelta(tool_call_id)
→ find exact ExecCall

ToolResult(tool_call_id)
→ complete exact ExecCall
```

If the matching call is not found:

```text
do not attach it to whichever ExecCell happens to be active
```

Treat it as a routing mismatch and materialize/recover a separate finalized execution entry as appropriate.

Never silently merge unrelated execution histories.

---

# 96. Full vs compact representation

ExecCell needs two render representations.

## Main Session transcript

Compact:

```text
◉ Running cargo test
    Compiling ...
    ...
    test xyz
```

or:

```text
• Explored
  └ Read app.rs, state.rs
    Search Queue in src
    List tests
```

or:

```text
• Ran 4 commands
```

The main viewport must stay readable.

## Full transcript/detail representation

Fuller:

```text
$ cargo test
<complete retained output>

✓ · 12.4s
```

or:

```text
✗ exit 101 · 12.4s
```

Main viewport compression and full transcript representation are separate rendering policies over the same ExecCell data.

---

# 97. Live output limits

ExecCell must never retain or render unbounded live output.

Use a bounded live preview with:

```text
byte limit
line limit
per-line byte limit
head preservation
tail preservation
partial current-line preservation
omitted-count marker
```

The exact constants are config/implementation choices.

The invariant is:

```text
live output memory is bounded
main viewport height is bounded
full canonical output authority stays upstream
```

The TUI does not need to become another persistence layer.

---

# 98. Duration and final result contract

Running elapsed time may be derived locally:

```text
now - started_at
```

while the call is live.

Final durable presentation should prefer canonical execution facts:

```text
duration
exit/result status
error/success
```

If those facts are not currently exposed by the normalized ToolResult/protocol but the product wants to render them after resume/reload, extend the canonical protocol.

Do not infer final exit code or duration from rendered stdout/stderr.

---

# 99. ExecCell target directory

ExecCell is now explicit in the target directory.

Recommended target once the current `transcript.rs` grows:

```text
features/thread/transcript/
├─ state.rs
├─ cell.rs
├─ projection.rs
├─ exec/
│  ├─ model.rs
│  ├─ live_output.rs
│  ├─ grouping.rs
│  ├─ render.rs
│  └─ *_tests.rs
└─ visualization/
   ├─ model.rs
   ├─ render.rs
   └─ *_tests.rs
```

If this is initially too many files, start with:

```text
features/thread/transcript.rs
features/thread/exec_cell.rs
```

and split only when implementation size requires it.

What is fixed is ownership:

```text
ExecCell
→ Thread transcript feature

not:
components/exec_cell as product state owner
```

Generic rendering helpers may still live below in components/ui where genuinely reusable.

---

# 100. v12 transcript model

```text
Thread transcript feature
│
├─ TranscriptProjection
│  └─ ordered TranscriptCells
│
├─ MessageCell
├─ ReasoningCell
├─ ExecCell
│  ├─ calls by ToolCallId
│  ├─ live output
│  ├─ SingleExec
│  ├─ ExploreGroup
│  ├─ CompactCommandGroup
│  ├─ completion/result
│  └─ compact/full rendering policy
├─ PlanCell
└─ ErrorCell
```

Lifecycle remains orthogonal:

```text
MessageCell  Live / Final
ReasoningCell Live / Final
ExecCell      Live / Final
...
```

`ExecCell` is not synonymous with `Live`.

`HistoryCell` is not synonymous with `Final`.


---

# 101. Transcript Cell Expansion

The transcript needs a first-class inline expand/collapse capability.

Name it:

```text
Cell Expansion
```

or, when more explicit:

```text
Transcript Cell Expansion
```

Do not call it `Inline Visualization`.

The capability means:

```text
Compact Cell
   ↓ toggle
Expanded Cell
```

Both are the same transcript cell.

Expansion changes the cell's height inside the normal transcript layout.

It does not open an overlay.

---

# 102. Expansion vs QuickView

These solve different problems.

```text
Cell Expansion
= stay inside transcript context
= show more of the same cell inline
= changes normal transcript height

QuickView
= inspect the content separately
= temporary overlay
= independent scrolling
= does not change normal layout height
```

Product rule:

> Expansion means "show me more here."  
> QuickView means "let me inspect the full thing separately."

Do not collapse these into one mechanism.

---

# 103. Three-level information model

Cells that need richer inspection may expose three levels:

```text
Compact
   ↓ toggle
Expanded
   ↓ view full/details
QuickView
```

Not every cell needs all three.

Example:

```text
ExecCell
→ Compact + Expanded + QuickView

ReasoningCell
→ Compact + Expanded
→ QuickView only when reasoning is very long

DiffCell
→ Compact + Expanded + QuickView

ToolCell
→ Compact + Expanded + QuickView

ErrorCell
→ Compact + Expanded
→ QuickView when diagnostics are large
```

---

# 104. ExecCell expansion

Compact:

```text
▸ Ran cargo test · 12.4s
```

Expanded:

```text
▾ Ran cargo test · 12.4s
  Compiling zeta...
  test queue::...
  ...
  428 passed
  view full
```

QuickView:

```text
┌──────── cargo test ──────────────────────────────┐
│ $ cargo test                                     │
│                                                  │
│ <full retained output, independently scrollable> │
│                                                  │
│ exit 0 · 12.4s                                   │
└──────────────────────────────────────────────────┘
```

Expanded ExecCell should still use a bounded preview.

QuickView is the place for the full retained canonical representation.

---

# 105. ReasoningCell expansion

Compact:

```text
▸ Thought for 18s
```

Expanded:

```text
▾ Thought for 18s
  Need to inspect the session model first...
  Queue ownership currently...
```

Normally no QuickView is necessary if the reasoning fits comfortably inline.

If reasoning is very large:

```text
Expanded
→ bounded preview

QuickView
→ full reasoning
```

Do not automatically give every ReasoningCell a separate detail overlay.

---

# 106. DiffCell expansion

Compact:

```text
▸ Changed 4 files · +120 -38
```

Expanded:

```text
▾ Changed 4 files · +120 -38
  src/app.rs        +40 -8
  src/state.rs      +31 -12
  src/frame.rs      +21 -5
  ...
  view full diff
```

QuickView:

```text
full diff
independent scroll
copy/inspection-oriented
```

---

# 107. ToolCell expansion

For non-Exec tools, a tool-specific cell may show:

Compact:

```text
▸ Called search
```

Expanded:

```text
▾ Called search
  query: "Queue"
  path: src/
```

QuickView:

```text
full arguments
full result
structured JSON/detail
```

Do not force every tool into ExecCell if it has fundamentally different presentation semantics.

---

# 108. ErrorCell expansion

Compact:

```text
▸ Build failed
```

Expanded:

```text
▾ Build failed
  error[E0308]: mismatched types
  ...
```

QuickView is appropriate only when diagnostics are large enough to benefit from dedicated scrolling/inspection.

---

# 109. Expansion state ownership

Expansion is TUI presentation state.

It is not canonical transcript truth.

Store by stable presentation-cell identity:

```text
ThreadPresentationState
└─ expanded_cells: Set<TranscriptCellId>
```

or an equivalent structure.

Never key expansion by rendered row index or assume one Cell equals one protocol `EntryId`.

Reason:

```text
resize
wrapping
live updates
prepend history
Exec grouping
```

can all move rows or combine several canonical entries into one visible cell.

Stable `TranscriptCellId` preserves the user's intent.

---

# 110. Cell capabilities

Do not add an `ExpandableCellManager`.

A cell only needs simple capabilities.

Conceptually:

```text
can_expand
has_details
```

Meaning:

```text
can_expand
→ has an inline expanded representation

has_details
→ has a QuickView/full-detail representation
```

They are independent.

Examples:

```text
MessageCell
can_expand = usually false
has_details = usually false

ExecCell
can_expand = true
has_details = true

ReasoningCell
can_expand = true
has_details = optional

DiffCell
can_expand = true
has_details = true
```

Exact API/trait shape is implementation-driven.

---

# 111. Mouse interaction

Both Expansion and QuickView should support mouse interaction.

But neither should require a mouse.

Use distinct click targets.

```text
click ▸ / ▾
→ expand / collapse

click "view full" / "details"
→ open QuickView
```

Avoid:

```text
click anywhere on cell
→ guess whether user wanted expansion or QuickView
```

Explicit affordances reduce accidental navigation.

---

# 112. Keyboard interaction

When transcript cell keyboard selection exists:

```text
Space
→ expand / collapse selected cell

Enter
→ open QuickView/details when available

Esc
→ close QuickView
```

Do not add a special global `Focus::ExpandableCell`.

If transcript selection becomes a persistent navigation mode, keep that state in the transcript/history presentation layer.

The selected cell remains identified by stable `TranscriptCellId`.

---

# 113. Expansion and scroll anchoring

Expansion changes normal transcript height.

Therefore scroll anchoring must preserve reading position as much as possible.

Prefer:

```text
Anchored {
    cell_id,
    line_offset,
}
```

When a cell above the current viewport expands or collapses:

```text
keep the anchor cell visually stable
```

Do not jump the user to the bottom.

Live cell updates follow the same rule when the user is not in FollowTail mode.

---

# 114. QuickView geometry for cell details

QuickView remains a generic overlay mechanism.

For cell detail:

```text
feature/cell decides content
QuickView decides geometry + scroll
```

Examples:

```text
ExecCell full retained canonical output
DiffCell full diff
ToolCell full JSON/result
ReasoningCell full text when necessary
ErrorCell full diagnostics
```

QuickView should use the available content area and clamp to terminal size.

Do not create one QuickView implementation per cell type.

---

# 115. When to expand and when to use QuickView

Use this rule:

```text
Can the additional content still be read comfortably
inside the normal transcript without dominating it?
→ Expansion

Is the content potentially tens/hundreds/thousands of lines,
or does it need dedicated scrolling/inspection?
→ QuickView
```

Typical guideline:

```text
roughly 5–15 useful extra lines
→ good Expansion territory

large output / full diff / full JSON / deep diagnostics
→ QuickView
```

This is a UX guideline, not a hard line-count contract.

---

# 116. Cell interaction summary

```text
Compact Cell
│
├─ click ▸ / keyboard Space
│      ↓
│   Expanded Cell
│
└─ click details / keyboard Enter
       ↓
    QuickView

Expanded Cell
│
├─ click ▾ / keyboard Space
│      ↓
│   Compact Cell
│
└─ click view full / keyboard Enter
       ↓
    QuickView
```

QuickView:

```text
Esc
→ close
→ return to the exact transcript state
```

---

# 117. Updated transcript target

```text
Thread transcript feature
│
├─ TranscriptProjection
│  └─ ordered TranscriptCells
│
├─ MessageCell
├─ ReasoningCell
├─ ExecCell
├─ DiffCell
├─ ToolCell
├─ InlineVisualizationCell
├─ PlanCell
├─ ErrorCell
└─ ...
        │
        ├─ Compact representation
        ├─ optional Expanded representation
        └─ optional QuickView details
```

Lifecycle remains separate:

```text
Live / Final
```

Presentation remains separate:

```text
Compact / Expanded
```

Detail surface remains separate:

```text
QuickView open / closed
```

Do not combine these three dimensions into one mega-state enum.

---

# 118. Final invariant

For any transcript cell, keep these concerns orthogonal:

```text
WHAT is it?
→ Message / Reasoning / Exec / Diff / Tool / Error / ...

IS it still changing?
→ Live / Final

HOW much is shown inline?
→ Compact / Expanded

IS full detail being inspected separately?
→ QuickView open / closed
```

This separation is the long-term transcript UI model.


---

# 119. `TranscriptCellId` is a presentation identity

`TranscriptCellId` is required because visible transcript cells are not always one-to-one with canonical entries.

Examples:

```text
Agent message entry
→ one MessageCell

Reasoning entry
→ one ReasoningCell

ToolCall + ToolOutput + ToolResult
→ one ExecCall

several related ExecCalls
→ one ExecCell
```

Therefore these are different concepts:

```text
EntryId
= canonical transcript entry identity

ItemId
= canonical Thread item identity

ToolCallId
= canonical execution/tool-call identity

TranscriptCellId
= stable TUI projection-cell identity
```

`TranscriptCellId` is presentation identity only.

Do not persist it as a new Core domain identifier unless another consumer genuinely needs it.

---

# 120. Derive `TranscriptCellId` deterministically

Do not generate a fresh random Cell ID every render or every resync.

The same canonical transcript should project to the same Cell IDs.

Recommended principle:

```text
single-entry cell
→ derive from canonical EntryId / ItemId identity

single ExecCell
→ derive from its first ToolCallId

grouped ExecCell
→ derive from the first ToolCallId in the stable group
```

Exact encoding is implementation detail.

The important invariant:

```text
group grows
→ existing TranscriptCellId does not change
```

This preserves:

```text
expanded state
selection
scroll anchor
mouse target continuity
```

A finalized group should not be arbitrarily regrouped on every redraw.

A full resync should apply the same deterministic grouping policy and recover the same visible Cell boundaries when canonical input is unchanged.

---

# 121. Canonical transcript → ExecCell projection

The execution projection must be explicit.

Canonical flow:

```text
ThreadTranscriptEntry::Item(ToolCall)
        │
        │ ToolCallId X
        ▼
      ExecCall X
        ▲
        │ ToolCallId X
ThreadTranscriptEntry::ToolOutput(stdout/stderr)
        ▲
        │ ToolCallId X
ThreadTranscriptEntry::Item(ToolResult)
```

Projection rule:

```text
ToolCall
→ create/find ExecCall(tool_call_id)

ToolOutput
→ append/replace live output on exact ExecCall(tool_call_id)

ToolResult
→ finalize exact ExecCall(tool_call_id)

ExecCall
→ optionally accepted into an existing ExecCell by grouping policy
```

The visible transcript should normally not render transient stdout/stderr as unrelated standalone cells when they belong to a known ExecCall.

Stable `ToolCallId` is the routing authority.

---

# 122. Orphan execution events

Never attach an execution update to "the current ExecCell" merely because it is current.

If:

```text
ToolOutput / ToolResult references ToolCallId X
```

and no projected ExecCall owns X:

```text
treat this as a routing mismatch
```

Then recover/materialize a separate execution representation from the canonical data available.

Do not merge it into another active call.

This preserves correctness when multiple executions are live or updates arrive after resync/recovery boundaries.

---

# 123. Canonical execution result metadata

The final execution display needs canonical result metadata.

Required product facts, when meaningful for the tool:

```text
success / failure
duration
exit code for process-like execution
terminal result/error metadata
```

Running elapsed time may be computed locally:

```text
now - local_started_at
```

but final/reloaded history must not depend on a TUI-local `Instant`.

If the normalized canonical tool result does not currently expose enough information to render:

```text
✓ · 12.4s
✗ exit 101 · 12.4s
```

after restart/resume, extend the Core/App Server protocol with the narrowest execution-result metadata required.

Do not parse stdout/stderr text to infer exit code or final duration.

Not every tool needs a numeric exit code.

The canonical contract should distinguish:

```text
generic tool success/failure
process execution exit status when applicable
```

without forcing shell semantics onto every tool.

---

# 124. QuickView "full" means full retained canonical representation

For transcript-cell detail, define:

```text
QuickView
= full retained canonical representation available to the TUI
```

This is not automatically equivalent to:

```text
every byte the underlying process ever produced
```

because upstream transcript accumulation may intentionally bound transient/output data.

If canonical data contains an omission marker, QuickView must preserve/show that fact.

Do not label truncated retained data as lossless "complete output."

If the product later requires truly complete large execution output, add an explicit upstream detail/output resource such as:

```text
fetch execution output by ToolCallId
```

or equivalent.

That capability belongs to Core/App Server storage/API design, not to an unbounded TUI buffer.

---

# 125. Transcript keyboard selection is presentation state

Mouse interaction can target disclosure/detail affordances directly.

Keyboard operation needs a selected transcript cell.

Conceptually:

```rust
struct TranscriptSelection {
    cell_id: TranscriptCellId,
}
```

This state belongs to the per-Thread transcript presentation layer.

It does not require:

```text
Focus::ExecCell
Focus::ReasoningCell
Focus::DiffCell
```

The exact key used to enter transcript-selection mode is a keymap/UX decision and can remain undecided until implementation.

Once selection exists:

```text
Space
→ expand/collapse

Enter
→ QuickView when details exist
```

The architecture requirement is the stable Cell selection identity, not a specific entry key.

---

# 126. v14 corrected transcript identity model

```text
CANONICAL
---------
EntryId
ItemId
ToolCallId
ThreadTranscriptEntry
        │
        ▼
THREAD TRANSCRIPT PROJECTION
----------------------------
ToolCallId X
├─ ToolCall
├─ ToolOutput stdout/stderr
└─ ToolResult
        │
        ▼
    ExecCall X
        │
        ├─ grouping policy
        ▼
    ExecCell
        │
        └─ TranscriptCellId
                │
                ├─ expanded_cells
                ├─ transcript selection
                └─ scroll anchor
```

For simple cells:

```text
canonical entry
→ TranscriptCell
→ deterministic TranscriptCellId
```

For grouped execution:

```text
many canonical entries
→ one/many ExecCalls
→ one ExecCell
→ one stable TranscriptCellId
```

This is the target identity boundary.


---

# 127. Vim editing is an accepted ChatInput capability

Vim editing is now part of the target TUI architecture.

It is not a product feature and must not live under:

```text
features/vim/
```

It belongs to the ChatInput editor mechanism:

```text
components/chat_input/
├─ editor.rs
├─ vim.rs
└─ completion/
```

Conceptually:

```text
ChatInput
└─ editor mode
   ├─ standard
   └─ vim
      ├─ Insert
      ├─ Normal
      └─ Visual
```

Exact supported Vim command coverage is a product/implementation decision.

The architectural boundary is fixed:

```text
Vim editing
= local text-editor behavior inside ChatInput
```

Do not let Vim mode become another application-wide state manager.

---

# 128. Vim input state

Vim-specific mutable state belongs to ChatInput.

Examples:

```text
mode
pending operator
visual anchor
count prefix
register/yank state when supported
```

Do not store this in `App`.

Conceptually:

```rust
struct ChatInputState {
    editor: EditorState,
    vim: Option<VimState>,
    completion: CompletionState,
    ...
}
```

Exact struct shape is implementation-driven.

---

# 129. Vim and completion priority

Vim and ChatInput completion must have an explicit local routing order.

Recommended principle:

```text
active completion popup
→ popup navigation/accept/cancel first

otherwise
→ Vim editor handles the key

otherwise
→ ordinary editor behavior
```

Examples:

```text
Insert mode + @foo
→ Mention completion may consume ↑/↓/Enter/Esc

Normal mode
→ j/k/h/l belong to Vim editor navigation

Esc with completion open
→ close completion first

Esc with no completion open in Insert mode
→ enter Normal mode
```

Keep this inside ChatInput input routing.

Do not route Vim keystrokes through a global application command bus.

---

# 130. Vim editing vs application navigation

ChatInput Vim editing and application-level navigation are separate concerns.

Do not infer:

```text
Vim enabled
→ every list/pane/transcript must use Vim keybindings
```

Possible future application-level aliases such as:

```text
j/k in Pane
h/l for root switching
```

belong to the global keymap/navigation policy.

They do not share the ChatInput Vim mode state machine.

This prevents:

```text
ChatInput Normal mode
```

from accidentally becoming a global application mode.

---

# 131. Vim configuration

Vim mode is presentation/config state.

A configuration may expose something like:

```text
tui.inputMode = standard | vim
```

Exact key naming follows the real config system.

The setting controls ChatInput editor behavior only.

Changing the setting must not alter canonical Session/Thread state.

---

# 132. `inline_visualization` is a terminal fallback capability

`inline_visualization` is now an accepted transcript capability.

Purpose:

> when the assistant produces an HTML/rich visualization for clients that can render it, the TUI presents a terminal-native fallback representation.

It is not:

```text
HTML rendering in Ratatui
a browser
Cell Expansion
QuickView
```

These are separate dimensions.

---

# 133. Visualization content model

The ideal canonical visualization artifact exposes both rich and fallback representations.

Conceptually:

```text
VisualizationArtifact
├─ rich/html representation
└─ terminal fallback semantics
```

For example:

```text
title
kind
structured data
text fallback
table/tree/chart-friendly semantic payload
```

Web/Desktop may render:

```text
rich/html
```

TUI renders:

```text
terminal fallback
```

The exact protocol shape must be defined by the canonical artifact/content owner.

---

# 134. Do not parse arbitrary HTML in the TUI

Reject this architecture:

```text
assistant HTML
     ↓
TUI HTML parser
     ↓
DOM/CSS interpretation
     ↓
Ratatui
```

The TUI should not become a partial browser engine.

Reasons:

```text
CSS/layout semantics do not map reliably to terminals
JavaScript cannot be safely/reliably reproduced
chart semantics are lost once reduced to arbitrary HTML
HTML parsing creates a second rendering platform inside the TUI
```

If only HTML is available and no semantic fallback exists, the safe terminal fallback is explicit:

```text
Visualization · terminal preview unavailable
```

with whatever metadata/link/artifact reference the canonical contract safely exposes.

---

# 135. `InlineVisualizationCell`

Visualization appears in the normal Thread transcript as:

```text
InlineVisualizationCell
```

It is one concrete TranscriptCell kind.

Example compact chart fallback:

```text
▸ Visualization · Build duration by package
```

Expanded:

```text
▾ Visualization · Build duration by package

  zeta-core         ███████████  12.4s
  app-server        ███████       8.1s
  tui               █████         5.7s

  view details
```

Another example:

```text
▾ Visualization · Dependency graph

  cli
   ├─ tui
   │  └─ app-server-client
   └─ ...
```

The rendering is terminal-native, not HTML emulation.

---

# 136. Visualization, Expansion and QuickView are orthogonal

Keep three questions separate:

```text
WHAT is the content?
→ InlineVisualizationCell

HOW much of it is shown inline?
→ Compact / Expanded

IS it being inspected separately?
→ QuickView open / closed
```

Therefore an InlineVisualizationCell may support:

```text
Compact
Expanded
QuickView
```

just like ExecCell or DiffCell.

Do not create a special Visualization overlay system.

---

# 137. When visualization uses Expansion

Use Expansion when the fallback can remain readable in the normal transcript.

Examples:

```text
small table
short bar chart
small dependency tree
few key metrics
short ASCII diagram
```

Compact:

```text
▸ Visualization · Test duration
```

Expanded:

```text
▾ Visualization · Test duration
  core  ██████  8.1s
  tui   ████    5.7s
```

This changes the normal transcript height.

---

# 138. When visualization uses QuickView

Use QuickView when terminal fallback is large or inspection-oriented.

Examples:

```text
large table
large tree
large ASCII diagram
long textual fallback
multi-screen structured result
```

QuickView provides:

```text
larger available terminal area
independent scrolling
stable inspection surface
```

It still renders the terminal fallback representation.

QuickView does not execute/render the original HTML.

---

# 139. Visualization fallback primitives

Generic terminal primitives may live below the transcript feature.

Examples:

```text
table layout
bar rendering
tree indentation
text wrapping
truncation
simple key/value grids
```

Possible ownership:

```text
features/thread/transcript/visualization/
→ understands visualization semantics and chooses representation

components/
→ reusable interactive/render mechanisms if needed

ui/
→ low-level product-agnostic drawing/text/layout helpers
```

Do not create:

```text
VisualizationManager
HtmlFallbackManager
VisualizationSurface
VisualizationRegistry
```

unless concrete implementation pressure proves a separate lifecycle exists.

---

# 140. Visualization identity and lifecycle

InlineVisualizationCell follows the same transcript identity rules as every other cell.

```text
canonical visualization entry/artifact
        ↓
InlineVisualizationCell
        ↓
TranscriptCellId
```

Its presentation state may include:

```text
expanded / collapsed
selection
QuickView open/closed
```

Those are TUI presentation facts.

The visualization artifact itself remains canonical upstream data.

---

# 141. Updated transcript cell taxonomy

Target:

```text
TranscriptCell
├─ MessageCell
├─ ReasoningCell
├─ ExecCell
├─ DiffCell
├─ ToolCell
├─ InlineVisualizationCell
├─ PlanCell
├─ ErrorCell
└─ ...
```

Not every canonical entry needs a distinct Rust type immediately.

This taxonomy describes meaningful presentation kinds.

Only create concrete specialized types when their rendering/state behavior is substantial enough to justify them.

---

# 142. Updated target directory

Relevant target additions:

```text
components/chat_input/
├─ editor.rs
├─ vim.rs
└─ completion/
   ├─ slash.rs
   ├─ mention.rs
   └─ skill.rs
```

and, when transcript code grows enough:

```text
features/thread/transcript/
├─ state.rs
├─ cell.rs
├─ projection.rs
├─ exec/
│  ├─ model.rs
│  ├─ live_output.rs
│  ├─ grouping.rs
│  └─ render.rs
└─ visualization/
   ├─ model.rs
   └─ render.rs
```

No top-level Vim feature.

No HTML renderer in the TUI.

No separate visualization surface manager.

---

# 143. v15 final capability model

```text
CHAT INPUT
----------
standard editor
or
Vim editor

Completion
├─ Slash
├─ Mention
└─ Skill


TRANSCRIPT
----------
TranscriptCell
├─ Message
├─ Reasoning
├─ Exec
├─ Diff
├─ Tool
├─ InlineVisualization
├─ Plan
└─ Error

Each cell independently has:
├─ Live / Final
├─ Compact / Expanded
└─ optional QuickView details


VISUALIZATION
-------------
rich/html artifact
        │
        ├─ rich client → HTML/rich renderer
        │
        └─ TUI → terminal fallback
                  ↓
          InlineVisualizationCell
                  ↓
          Compact / Expanded / QuickView
```

This keeps editor behavior, transcript presentation and rich-artifact fallback as separate architecture concerns.
