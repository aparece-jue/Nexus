# Worktrace source attribution

Source: the user's Worktrace project, fixed source commit 7b9cd47ee7e1b5d41a8a1403c9cbb21d9a6feb62, locally inspected under .local-validation/worktrace-7b9cd47. No upstream URL is asserted here.

prototypes/knowledge-store/src/task_status.rs adapts the TaskStatus enum and implementation from src-tauri/src/domain/task.rs. The state strings and transition target table are retained. The source import, error coupling and serialization implementation are omitted; current serialization is handled by the catalog response. The upstream license is retained in Worktrace-LICENSE (Apache-2.0). The inspected source has no separate NOTICE file. No complete dependency license audit or release clearance is claimed.
