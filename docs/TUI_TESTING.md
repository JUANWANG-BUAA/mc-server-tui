# TUI feedback and test loop

The TUI uses the daemon API and durable task event cursor; rendering and input handling stay separate from daemon I/O. This makes the UI testable without contacting Paper or waiting on real downloads.

## Automated checks

1. Keep screen state and wizard input separate from rendering and daemon I/O. State tests cover field validation, back navigation with retained values, explicit EULA acceptance, queued task success, retry after a submission error, monotonic event cursors, duplicate event replay, and retaining the last good detail after a failed refresh.
2. Render dashboard, wizard, and task detail states to an in-memory terminal backend. Cover 80×24 reference layouts and 40×12 compact layouts, including all wizard steps, the EULA control, task progress, and the event cursor.
3. Run input and polling together against fake daemons that delay replies or emit ordered task events. Assert that navigation, wizard text input, and Ctrl-C remain responsive while a request is pending, task detail resumes with `after_sequence` after the last received event when reopened, and a failed request does not erase the last good frame.
4. Use a PTY smoke test for raw-mode entry, resize, key handling, and terminal restoration on normal exit and panic. Keep this separate from view-model tests because terminal ownership is process-level behavior.

## Visual review

For a release candidate, run the TUI in a real terminal at the reference sizes, drive the fake daemon through each task state, and capture terminal screenshots. Review hierarchy, truncation, contrast, progress indication, and resize behavior against the same fixtures used by the renderer tests. A screenshot is a review artifact; the deterministic cell snapshot remains the CI regression check.

The dashboard, creation wizard, and task detail flow are implemented. Task detail fetches the task snapshot and events concurrently, polls after the last received event sequence, deduplicates events, and preserves its cursor when closed and reopened. `TestBackend` tests cover dashboard, wizard, and task detail at 80×24 and 40×12. Run `cargo build -p mc-tui && python3 scripts/tui_pty_smoke.py && python3 scripts/tui_task_detail_pty.py`; the first PTY test stalls creation to verify input/exit responsiveness, while the second drives event sequences 1→2→3 and checks reopening resumes at 3. Real-terminal screenshot review, instance detail, and lifecycle actions remain open.
