# TUI feedback and test loop

The TUI uses the daemon API and durable task event cursor; rendering and input handling stay separate from daemon I/O. This makes the UI testable without contacting Paper or waiting on real downloads.

## Automated checks

1. Keep screen state and wizard input separate from rendering and daemon I/O. State tests cover field validation, back navigation with retained values, explicit EULA acceptance, queued task success, and retry after a submission error.
2. Render dashboard and wizard states to an in-memory terminal backend. Cover 80×24 reference layouts and 40×12 compact layouts, including all wizard steps and the EULA control. Dashboard task fixtures cover queued, resolving, downloading, installing, succeeded, failed, and interrupted states as those views are added.
3. Run input and polling together against a fake daemon that can delay replies and emit ordered task events. Assert that navigation, wizard text input, and Ctrl-C remain responsive while a request is pending, reconnect resumes after the last event sequence, and a failed request does not erase the last good frame.
4. Use a PTY smoke test for raw-mode entry, resize, key handling, and terminal restoration on normal exit and panic. Keep this separate from view-model tests because terminal ownership is process-level behavior.

## Visual review

For a release candidate, run the TUI in a real terminal at the reference sizes, drive the fake daemon through each task state, and capture terminal screenshots. Review hierarchy, truncation, contrast, progress indication, and resize behavior against the same fixtures used by the renderer tests. A screenshot is a review artifact; the deterministic cell snapshot remains the CI regression check.

The dashboard and first creation flow are implemented: an instance/task dashboard with two-second asynchronous refresh, manual refresh, panel navigation, and a three-step Paper wizard. The wizard validates server/runtime fields, requires explicit EULA acceptance, submits through the daemon in a Tokio task, keeps input responsive while waiting, and retains form values after a submission error. `TestBackend` tests cover dashboard and wizard layouts at 80×24 and 40×12. Run the delayed fake-daemon check with `cargo build -p mc-tui && python3 scripts/tui_pty_smoke.py`; it verifies both list RPCs can remain pending while the wizard reaches review and submits a create request, Ctrl-C exits while that request is stalled, and the alternate screen is restored. Reconnect cursor tests, real-terminal screenshot review, instance detail/error screens, and lifecycle actions remain open.
