# Editor subsystems

This directory owns reusable editor implementations. Product directories own
only static composition: Code imports the Monaco contribution, Academic imports
the ProseMirror contribution, and Complete imports both editor contributions.

Each editor follows the same customization boundary:

- `common/` owns DOM-independent input matching, models, schemas, and policy;
- `browser/` owns DOM integration, workers, layout, focus, styling, and cleanup;
- `contrib/` registers the editor descriptor with the shared Workbench;
- `test/` keeps subsystem contracts next to the implementation they protect.

Do not move implementation into the product entry modules under `src/code` or
into the shared Workbench. Product entry points may select an editor
contribution but must not own its schema, model, plugins, workers, or lifecycle.
