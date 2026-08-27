# zeta-editor-host

Editor-specific retained state and auxiliary presentation: find/replace, selection auto-scroll,
diagnostics, and language-service popovers.

The app host still owns files, Tabs, persistence, save conflicts, App Server requests, and event
routing. This crate receives typed editor/LSP results and resolved colors.
