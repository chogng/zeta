# zeta-session

1. Owns the App Server Session runtime: connection worker, Session/Thread requests and subscriptions, bounded command queue, remote reconnect policy, and backend-assembled transcript delivery.
2. Owns one Agent Session Pane: Thread metadata, transcript state, timeline scroll, Composer state, layout, painting, accessibility, and internal interactions.
3. Accepts connection targets, styles, and host effects; files, Git, configuration, Tabs, windows, and product event routing remain with their domain crates or the product host.
