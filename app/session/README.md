# zeta-session

1. Owns one Agent Session Pane: canonical Thread metadata, backend-assembled transcript entries, timeline scroll, Composer state, and their synchronization.
2. Owns that Pane's layout, painting, input context toolbar, accessibility nodes, and internal interactions.
3. Mechanically applies transcript changes and accepts catalogs, styles, and host effects; transcript assembly, Tabs, Tab search, Tab context menus, Tab switching, windows, and App Server transport remain outside this crate.
