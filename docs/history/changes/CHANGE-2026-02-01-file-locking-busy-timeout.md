# CHANGE-2026-02-01-file-locking-busy-timeout

Milestone: M1
Summary:
- Store access now uses a lock file to enforce single-writer, multi-reader semantics.
- SQLite connections apply a busy timeout to reduce immediate lock failures.

User impact:
- Concurrent read operations can proceed; conflicting writes surface a clear lock error.

Migration:
- None.

References:
- ISSUE-2026-02-01-file-locking-busy-timeout
