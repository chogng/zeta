# zeta-async-utils

`zeta-async-utils` provides executor-independent cooperative cancellation for Rust futures and
structured parent/child work such as multi-agent runs. It has no runtime or third-party
dependencies.

## Cancellation domains

A `CancellationSource` owns cancellation authority. Its `CancellationToken` is read-only:

```rust
use zeta_async_utils::{CancellationSource, FutureCancellationExt};

let source = CancellationSource::new();
let token = source.token();

let result = do_work()
    .with_cancellation(token)
    .await;
```

The wrapper pins the inner future in a box, then checks cancellation before polling it. When
cancellation wins, it drops the inner future and returns `Err(Cancellation)`. Use this only when
dropping the future at an await point is safe.

Work that must perform asynchronous cleanup should observe cancellation itself:

```rust
# use zeta_async_utils::CancellationToken;
# async fn next_step() {}
async fn worker(token: CancellationToken) {
    loop {
        if let Err(cancellation) = token.check() {
            // Flush, rollback, or release resources here.
            eprintln!("{cancellation}");
            return;
        }
        next_step().await;
    }
}
```

`token.cancelled().await` is also available for event-driven loops and custom `select` mechanisms.

## Parent and child agents

Create a distinct child source before spawning each child agent:

```rust
use zeta_async_utils::CancellationSource;

let parent = CancellationSource::new();
let child_a = parent.token().child_source();
let child_b = parent.token().child_source();

parent.cancel(); // cancels parent, child_a, child_b, and their descendants
```

The hierarchy has these guarantees:

- Parent cancellation propagates to all currently live descendants.
- A child created after its parent is cancelled starts cancelled with the same signal.
- Child cancellation does not affect its parent or siblings.
- Cloning a source shares one cancellation domain; it does not create a child.
- The first signal observed by each domain wins. The signal retains its original domain ID and
  application-defined reason as it propagates.
- Parents retain weak references to children, so completed child domains are not kept alive.
- Propagation is iterative and waiter registration is synchronized with cancellation, preventing
  stack overflow and missed wakeups.

For custom reasons, parameterize the source:

```rust
#[derive(Debug)]
enum StopReason {
    UserInterrupted,
    ParentFailed,
}

let source = CancellationSource::<StopReason>::new_typed();
source.cancel_with(StopReason::UserInterrupted);
```

## Lifecycle behavior

Dropping a source does not implicitly cancel its tokens. This avoids surprising cancellation when
one of several source clones is dropped. An owner that needs scope-exit cancellation can hold a
`CancelOnDrop` guard:

```rust
use zeta_async_utils::CancellationSource;

let source = CancellationSource::new();
let guard = source.cancel_on_drop();
// Dropping `guard` requests cancellation, including on early return or unwind.
# guard.disarm();
```

Cancellation cannot interrupt blocking synchronous code, an operating-system call, or independently
spawned work that does not observe a related token. Such work must use explicit checkpoints or its
own runtime-specific interruption mechanism.
