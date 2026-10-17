# TUI feedback and test loop

The TUI will use the daemon API and durable task event cursor; rendering and input handling stay separate from daemon I/O. This makes the UI testable without contacting Paper or waiting on real downloads.

## Automated checks

1. Keep screen state as a view model built from instance/task snapshots and events. Unit tests feed queued, resolving, downloading, installing, succeeded, failed, and interrupted fixtures into the view model.
2. Render those fixtures to an in-memory terminal backend and compare cell snapshots at 80×24, 100×30, and 40×12. Include empty lists, long names, error text, selection, and clipped layouts.
3. Run input and polling together against a fake daemon that can delay replies and emit ordered task events. Assert that navigation and quit keys remain responsive while a poll is pending, reconnect resumes after the last event sequence, and a failed request does not erase the last good frame.
4. Use a PTY smoke test for raw-mode entry, resize, key handling, and terminal restoration on normal exit and panic. Keep this separate from view-model tests because terminal ownership is process-level behavior.

## Visual review

For a release candidate, run the TUI in a real terminal at the reference sizes, drive the fake daemon through each task state, and capture terminal screenshots. Review hierarchy, truncation, contrast, progress indication, and resize behavior against the same fixtures used by the renderer tests. A screenshot is a review artifact; the deterministic cell snapshot remains the CI regression check.

The current repository has the daemon task API and CLI inspector, but no TUI implementation yet. Once the TUI lands, the first UI milestone should add the in-memory renderer and fake-daemon loop before styling the full screen.
