# zeta-agent-session

Typed boundary for the app-side Agent Session worker.

The app host still owns App Server connections, worker lifecycle, files, Git, language-service
side effects, and window events. This crate owns the command/event vocabulary, bounded worker
queue, and reusable reconnect timing/rejection policy so those responsibilities do not leak into
feature views.
