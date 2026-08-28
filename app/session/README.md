# zeta-session

1. Owns the App Server Session runtime: connection worker, Session/Thread requests and subscriptions, bounded command queue, remote reconnect policy, and backend-assembled transcript delivery.
2. Owns one Agent Session Pane: Thread metadata and header wrap a ChatWidget containing the transcript timeline and ChatInput; ChatInput owns its editor, toolbar, and the input-only interaction area used by Slash Commands and model selection.
3. Accepts connection targets, styles, and host effects; files, Git, configuration, Tabs, windows, and product event routing remain with their domain crates or the product host.
