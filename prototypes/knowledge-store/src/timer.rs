//! Persistence kernel only. Caller must supply verified clock samples; not an IPC API.
//! Millisecond precision, paused/recovering exclusion follow Worktrace 7b9cd47 semantics.
use crate::{Result, Store};
use rusqlite::params;
use serde::Serialize;

pub(crate) fn ensure_task_not_timed(db: &rusqlite::Connection, task: &str) -> Result<()> {
    let active: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM work_session WHERE task_id=?1 AND state<>'finished')",
        [task],
        |r| r.get(0),
    )?;
    if active {
        return Err("请先结束或处置该任务的计时会话，再修改任务或添加子任务".into());
    }
    Ok(())
}

pub(crate) const SCHEMA: &str = "
CREATE TABLE work_session(id TEXT PRIMARY KEY,task_id TEXT NOT NULL REFERENCES task(id),state TEXT NOT NULL CHECK(state IN ('running','paused','recovering','finished')),row_version INTEGER NOT NULL DEFAULT 1,last_at_ms INTEGER NOT NULL);
CREATE UNIQUE INDEX single_active_session ON work_session((1)) WHERE state<>'finished';
CREATE TABLE work_interval(session_id TEXT NOT NULL REFERENCES work_session(id),sequence INTEGER NOT NULL,start_ms INTEGER NOT NULL,end_ms INTEGER,needs_review INTEGER NOT NULL DEFAULT 0 CHECK(needs_review IN (0,1)),CHECK(end_ms IS NULL OR end_ms>=start_ms),PRIMARY KEY(session_id,sequence));
PRAGMA user_version=5;";

#[derive(Serialize)]
pub struct TimerSnapshot {
    pub id: String,
    pub task_id: String,
    pub state: String,
    pub row_version: i64,
    pub confirmed_ms: i64,
    pub pending_intervals: i64,
}

impl Store {
    /// Release the active slot without turning an unknown interval into confirmed work.
    /// Pending evidence remains queryable; this is not a duration correction.
    pub fn finish_recovery(&mut self, id: &str, expected: i64) -> Result<()> {
        if expected < 1 || expected == i64::MAX {
            return Err("无效版本".into());
        }
        let changed=self.db.execute("UPDATE work_session SET state='finished',row_version=row_version+1 WHERE id=?1 AND row_version=?2 AND state='recovering'",params![id,expected])?;
        if changed != 1 {
            return Err("恢复会话已变化，请刷新后重试".into());
        }
        Ok(())
    }
    pub fn start_timer(&mut self, id: &str, task: &str, at_ms: i64) -> Result<()> {
        if id.trim().is_empty() || at_ms < 0 {
            return Err("无效会话或时钟".into());
        }
        let tx = self.db.transaction()?;
        let eligible:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM task WHERE id=?1 AND archived=0 AND status IN ('Ready','Doing') AND NOT EXISTS(SELECT 1 FROM task child WHERE child.parent_id=task.id))",[task],|r|r.get(0))?;
        if !eligible {
            return Err("仅未归档的待办或进行中叶任务可计时".into());
        }
        tx.execute(
            "INSERT INTO work_session(id,task_id,state,last_at_ms) VALUES (?1,?2,'running',?3)",
            params![id, task, at_ms],
        )?;
        tx.execute(
            "INSERT INTO work_interval(session_id,sequence,start_ms) VALUES (?1,1,?2)",
            params![id, at_ms],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn timer_snapshot(&self, id: &str) -> Result<TimerSnapshot> {
        let mut snapshot = self.db.query_row(
            "SELECT id,task_id,state,row_version FROM work_session WHERE id=?1",
            [id],
            |r| {
                Ok(TimerSnapshot {
                    id: r.get(0)?,
                    task_id: r.get(1)?,
                    state: r.get(2)?,
                    row_version: r.get(3)?,
                    confirmed_ms: 0,
                    pending_intervals: 0,
                })
            },
        )?;
        let mut query=self.db.prepare("SELECT start_ms,end_ms,needs_review FROM work_interval WHERE session_id=?1 ORDER BY sequence")?;
        let rows = query.query_map([id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, Option<i64>>(1)?,
                r.get::<_, bool>(2)?,
            ))
        })?;
        for row in rows {
            let (start, end, pending) = row?;
            if pending {
                snapshot.pending_intervals += 1;
                continue;
            }
            if let Some(end) = end {
                let duration = end
                    .checked_sub(start)
                    .filter(|value| *value >= 0)
                    .ok_or("无效区间")?;
                snapshot.confirmed_ms = snapshot
                    .confirmed_ms
                    .checked_add(duration)
                    .ok_or("工时溢出")?;
            }
        }
        Ok(snapshot)
    }
    pub fn change_timer(
        &mut self,
        id: &str,
        expected: i64,
        action: &str,
        at_ms: i64,
    ) -> Result<()> {
        if expected < 1 || expected == i64::MAX || at_ms < 0 {
            return Err("无效版本或时钟".into());
        }
        let tx = self.db.transaction()?;
        let (state, version, last): (String, i64, i64) = tx.query_row(
            "SELECT state,row_version,last_at_ms FROM work_session WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        if version != expected {
            return Err("计时版本冲突".into());
        }
        if at_ms < last {
            return Err("时钟回退，拒绝确认区间".into());
        }
        let next = match (state.as_str(), action) {
            ("running", "pause") => "paused",
            ("paused", "resume") => "running",
            ("running" | "paused", "finish") => "finished",
            _ => return Err("不允许此计时操作".into()),
        };
        if state == "running" {
            let changed=tx.execute("UPDATE work_interval SET end_ms=?2 WHERE session_id=?1 AND end_ms IS NULL AND needs_review=0",params![id,at_ms])?;
            if changed != 1 {
                return Err("开放区间异常".into());
            }
        }
        if action == "resume" {
            tx.execute("INSERT INTO work_interval(session_id,sequence,start_ms) SELECT ?1,COALESCE(MAX(sequence),0)+1,?2 FROM work_interval WHERE session_id=?1",params![id,at_ms])?;
        }
        tx.execute(
            "UPDATE work_session SET state=?2,row_version=row_version+1,last_at_ms=?3 WHERE id=?1",
            params![id, next, at_ms],
        )?;
        tx.commit()?;
        Ok(())
    }
    /// Call only after acquiring exclusive workspace ownership, before accepting timer commands.
    pub fn recover_interrupted_timers(&mut self) -> Result<()> {
        let tx = self.db.transaction()?;
        tx.execute("UPDATE work_interval SET needs_review=1 WHERE end_ms IS NULL AND session_id IN (SELECT id FROM work_session WHERE state='running')",[])?;
        tx.execute("UPDATE work_session SET state='recovering',row_version=row_version+1 WHERE state='running'",[])?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn task(store: &mut Store, id: &str) -> Result<()> {
        store.create_task(id, id, None, None, None, &[])?;
        store.transition_task(id, 1, "Ready", false)
    }
    #[test]
    fn active_and_recovering_sessions_protect_task_then_release_without_confirming_unknown_time(
    ) -> Result<()> {
        use crate::tasks::TaskFields;
        let dir = tempfile::tempdir()?;
        let mut store = Store::open(dir.path().to_owned())?;
        task(&mut store, "task")?;
        store.start_timer("one", "task", 0)?;
        let fields = TaskFields {
            title: "changed".into(),
            project_id: None,
            parent_id: None,
            tags: vec!["tag".into()],
        };
        assert!(store.update_task("task", 2, &fields, true).is_err());
        assert!(store.transition_task("task", 2, "Done", false).is_err());
        assert!(store
            .create_task("child", "child", None, None, Some("task"), &[])
            .is_err());
        store.change_timer("one", 1, "pause", 1000)?;
        assert!(store.update_task("task", 2, &fields, false).is_err());
        store.change_timer("one", 2, "resume", 2000)?;
        drop(store);
        let mut store = Store::open(dir.path().to_owned())?;
        store.recover_interrupted_timers()?;
        assert!(store.update_task("task", 2, &fields, false).is_err());
        assert!(store.finish_recovery("one", 3).is_err());
        store.finish_recovery("one", 4)?;
        assert!(store.finish_recovery("one", 4).is_err());
        let snapshot = store.timer_snapshot("one")?;
        assert_eq!(snapshot.confirmed_ms, 1000);
        assert_eq!(snapshot.pending_intervals, 1);
        store.start_timer("two", "task", 3000)?;
        store.change_timer("two", 1, "finish", 4000)?;
        store.update_task("task", 2, &fields, false)?;
        Ok(())
    }
    #[test]
    fn pause_exclusion_duplicate_conflict_and_reopen_preserve_thirty_five_minutes() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let mut store = Store::open(dir.path().to_owned())?;
        task(&mut store, "task")?;
        store.start_timer("session", "task", 0)?;
        assert!(store.start_timer("duplicate", "task", 0).is_err());
        store.change_timer("session", 1, "pause", 20 * 60_000)?;
        assert!(store
            .change_timer("session", 1, "pause", 20 * 60_000)
            .is_err());
        store.change_timer("session", 2, "resume", 30 * 60_000)?;
        assert!(store
            .change_timer("session", 3, "finish", 29 * 60_000)
            .is_err());
        store.change_timer("session", 3, "finish", 45 * 60_000)?;
        drop(store);
        let store = Store::open(dir.path().to_owned())?;
        let snapshot = store.timer_snapshot("session")?;
        assert_eq!(snapshot.confirmed_ms, 35 * 60_000);
        assert_eq!(snapshot.state, "finished");
        assert_eq!(snapshot.pending_intervals, 0);
        Ok(())
    }
    #[test]
    fn interruption_never_confirms_unknown_time_and_recovery_is_repeat_safe() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let mut store = Store::open(dir.path().to_owned())?;
        task(&mut store, "task")?;
        store.start_timer("session", "task", 0)?;
        store.change_timer("session", 1, "pause", 1000)?;
        store.change_timer("session", 2, "resume", 2000)?;
        drop(store);
        let mut store = Store::open(dir.path().to_owned())?;
        store.recover_interrupted_timers()?;
        store.recover_interrupted_timers()?;
        let snapshot = store.timer_snapshot("session")?;
        assert_eq!(snapshot.confirmed_ms, 1000);
        assert_eq!(snapshot.pending_intervals, 1);
        assert_eq!(snapshot.state, "recovering");
        assert_eq!(snapshot.row_version, 4);
        assert!(store.change_timer("session", 4, "finish", 9000).is_err());
        Ok(())
    }
}
