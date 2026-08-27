# zeta-remote-ui

Remote connection picker, connection manager, and Tunnel manager state and presentation.

The app host supplies the profile catalog, runtime launch, SSH process ownership, clipboard, and
window/event routing. This crate receives resolved UI style and typed Tunnel lifecycle events and
returns state transitions or user actions.
