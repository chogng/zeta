# `zeta-workbench-layout`

`zeta-workbench-layout` owns the structural geometry of the Workbench. It consumes Workbench-owned
logical topology together with a viewport and sizing policies, then returns immutable bounds,
sashes, and resize snapshots for one frame.

The crate may use generic geometry algorithms from `zui`, but it does not own product content,
runtime bindings, interaction dispatch, rendering, or frame scheduling. The product host remains
responsible for mapping the returned leaves to stable identities and feature presentations.

## Public contract

- `WorkbenchLayoutSpec` resolves the titlebar, Tab Container, main workspace, and Inspector bounds.
- `PaneGroupLayout` projects a `zeta_workbench::PaneNode` into generic Grid leaves and sashes.
- `TabContainerLayoutSpec` and `InspectorLayoutSpec` carry host-provided sizing policy only.
- `LogicalViewport` converts physical window dimensions into logical UI dimensions.

Layout resolution is pure with respect to Workbench state: it does not mutate the model or retain
runtime resources.

## Validation

Run `cargo test -p zeta-workbench-layout` for structural layout behavior and
`python3 -B build/cargo_with_v8.py check -p app` for product integration.
