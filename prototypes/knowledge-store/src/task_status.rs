// Adapted from Worktrace 7b9cd47 domain/task.rs: unchanged status table. Nexus validates transitions in tasks.rs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskStatus {
    Inbox,
    Clarifying,
    Ready,
    Scheduled,
    Doing,
    Blocked,
    Waiting,
    Review,
    Done,
    Cancelled,
}

impl TaskStatus {
    pub const ALL: [TaskStatus; 10] = [
        Self::Inbox,
        Self::Clarifying,
        Self::Ready,
        Self::Scheduled,
        Self::Doing,
        Self::Blocked,
        Self::Waiting,
        Self::Review,
        Self::Done,
        Self::Cancelled,
    ];

    /// 落库用的字符串。与 schema 的 CHECK 取值逐字一致。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Inbox => "Inbox",
            Self::Clarifying => "Clarifying",
            Self::Ready => "Ready",
            Self::Scheduled => "Scheduled",
            Self::Doing => "Doing",
            Self::Blocked => "Blocked",
            Self::Waiting => "Waiting",
            Self::Review => "Review",
            Self::Done => "Done",
            Self::Cancelled => "Cancelled",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.as_str() == s)
    }

    /// 终结态：只能经显式 reopen 离开。
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Done | Self::Cancelled)
    }

    /// V0.1 是否接受写入。`Scheduled` 属 V0.2（F-105）。
    pub fn is_writable_in_v01(self) -> bool {
        !matches!(self, Self::Scheduled)
    }

    /// 按 02 §5 的跃迁表判断允许性（**不含**「显式 reopen」这一附加条件，
    /// 那个由 [`TaskTransition::new`] 结合 cause 判断）。
    pub fn allowed_targets(self) -> &'static [TaskStatus] {
        use TaskStatus::*;
        match self {
            Inbox => &[Clarifying, Ready, Cancelled],
            Clarifying => &[Inbox, Ready, Cancelled],
            Ready => &[Doing, Scheduled, Blocked, Waiting, Review, Done, Cancelled],
            Scheduled => &[Ready, Doing, Blocked, Waiting, Review, Done, Cancelled],
            Doing => &[Ready, Blocked, Waiting, Review, Done, Cancelled],
            Blocked | Waiting => &[Ready, Cancelled],
            Review => &[Ready, Done, Cancelled],
            Done | Cancelled => &[Ready],
        }
    }
}
