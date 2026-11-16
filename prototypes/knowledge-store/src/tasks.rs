//! Office catalog increment. State names/targets reuse Worktrace 7b9cd47.
use crate::{task_status::TaskStatus, Result, Store};
use rusqlite::params;
use serde::Serialize;

pub(crate) const SCHEMA: &str = "
CREATE TABLE project(id TEXT PRIMARY KEY, name TEXT NOT NULL, row_version INTEGER NOT NULL DEFAULT 1);
CREATE TABLE task(id TEXT PRIMARY KEY, title TEXT NOT NULL, project_id TEXT REFERENCES project(id), status TEXT NOT NULL,
row_version INTEGER NOT NULL DEFAULT 1, source_note_id TEXT, source_revision INTEGER,
FOREIGN KEY(source_note_id,source_revision) REFERENCES version(item_id,revision),
CHECK((source_note_id IS NULL)=(source_revision IS NULL)));
PRAGMA user_version=3;";

#[derive(Serialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub row_version: i64,
}
#[derive(Serialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub project_id: Option<String>,
    pub status: String,
    pub row_version: i64,
    pub source_note_id: Option<String>,
    pub source_revision: Option<i64>,
    pub allowed_targets: Vec<String>,
    pub terminal: bool,
}
fn text(value: &str) -> Result<&str> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 200 {
        return Err("名称不能为空或超过200字".into());
    }
    Ok(value)
}
impl Store {
    pub fn create_project(&mut self, id: &str, name: &str) -> Result<()> {
        self.db.execute(
            "INSERT INTO project(id,name) VALUES (?1,?2)",
            params![text(id)?, text(name)?],
        )?;
        Ok(())
    }
    pub fn list_projects(&self) -> Result<Vec<Project>> {
        let mut query = self
            .db
            .prepare("SELECT id,name,row_version FROM project ORDER BY name,id")?;
        let rows = query.query_map([], |r| {
            Ok(Project {
                id: r.get(0)?,
                name: r.get(1)?,
                row_version: r.get(2)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
    pub fn create_task(
        &mut self,
        id: &str,
        title: &str,
        project: Option<&str>,
        source: Option<(&str, i64)>,
    ) -> Result<()> {
        let title = text(title)?;
        let id = text(id)?;
        if let Some((note, revision)) = source {
            self.read(note, revision)?;
        }
        let tx = self.db.transaction()?;
        if let Some((note, revision)) = source {
            let current: (i64, bool) = tx.query_row(
                "SELECT revision,archived FROM item WHERE id=?1",
                [note],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            if current != (revision, false) {
                return Err("来源便签已修改或归档，请重新读取".into());
            }
        }
        tx.execute("INSERT INTO task(id,title,project_id,status,source_note_id,source_revision) VALUES (?1,?2,?3,'Inbox',?4,?5)",params![id,title,project,source.map(|s|s.0),source.map(|s|s.1)])?;
        tx.commit()?;
        Ok(())
    }
    pub fn list_tasks(&self) -> Result<Vec<Task>> {
        let mut query = self.db.prepare("SELECT id,title,project_id,status,row_version,source_note_id,source_revision FROM task ORDER BY rowid DESC")?;
        let rows = query.query_map([], |r| {
            Ok(Task {
                id: r.get(0)?,
                title: r.get(1)?,
                project_id: r.get(2)?,
                status: r.get(3)?,
                row_version: r.get(4)?,
                source_note_id: r.get(5)?,
                source_revision: r.get(6)?,
                allowed_targets: Vec::new(),
                terminal: false,
            })
        })?;
        let mut tasks = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        for task in &mut tasks {
            let status = TaskStatus::parse(&task.status).ok_or("未知任务状态")?;
            task.terminal = status.is_terminal();
            task.allowed_targets = status
                .allowed_targets()
                .iter()
                .filter(|s| s.is_writable_in_v01())
                .map(|s| s.as_str().to_string())
                .collect();
        }
        Ok(tasks)
    }
    pub fn transition_task(
        &mut self,
        id: &str,
        expected: i64,
        target: &str,
        reopen: bool,
    ) -> Result<()> {
        if expected < 1 || expected == i64::MAX {
            return Err("无效版本".into());
        }
        let target = TaskStatus::parse(target).ok_or("未知目标状态")?;
        let tx = self.db.transaction()?;
        let current: (String, i64) = tx.query_row(
            "SELECT status,row_version FROM task WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if current.1 != expected {
            return Err("任务已修改，请刷新后重试".into());
        }
        let from = TaskStatus::parse(&current.0).ok_or("未知任务状态")?;
        if !target.is_writable_in_v01()
            || !from.allowed_targets().contains(&target)
            || (from.is_terminal() && !reopen)
            || (reopen && !from.is_terminal())
        {
            return Err("不允许此状态变更；已完成任务请显式重开".into());
        }
        let changed = tx.execute(
            "UPDATE task SET status=?3,row_version=row_version+1 WHERE id=?1 AND row_version=?2",
            params![id, expected, target.as_str()],
        )?;
        if changed != 1 {
            return Err("任务版本冲突".into());
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn schema_two_upgrade_keeps_archived_notes_and_a_schema_two_snapshot() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let db = rusqlite::Connection::open(directory.path().join("knowledge.db"))?;
        db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE item(id TEXT PRIMARY KEY,revision INTEGER NOT NULL,archived INTEGER NOT NULL DEFAULT 0); CREATE TABLE version(item_id TEXT NOT NULL REFERENCES item(id),revision INTEGER NOT NULL,body_hash TEXT NOT NULL,attachment_hash TEXT,PRIMARY KEY(item_id,revision)); PRAGMA user_version=2;")?;
        std::fs::create_dir(directory.path().join("blobs"))?;
        let mut previous = Store {
            root: directory.path().to_owned(),
            db,
        };
        previous.save("legacy", 0, b"legacy body", None)?;
        previous.set_note_archived("legacy", 1, true)?;
        drop(previous);
        let mut store = Store::open(directory.path().to_owned())?;
        assert!(store.is_note_archived("legacy")?);
        assert_eq!(store.read("legacy", 2)?, b"legacy body");
        assert!(store
            .create_task("t", "rejected archived source", None, Some(("legacy", 2)))
            .is_err());
        assert!(store.list_tasks()?.is_empty());
        let snapshot = rusqlite::Connection::open(directory.path().join("before-schema-3.db"))?;
        assert_eq!(
            snapshot.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))?,
            2
        );
        Ok(())
    }
    #[test]
    fn note_project_task_transition_reopen_and_reopen_database() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let mut store = Store::open(directory.path().to_owned())?;
        store.save("note", 0, b"source v1", None)?;
        store.create_project("project", "研发项目")?;
        store.create_task("task", "评估方案", Some("project"), Some(("note", 1)))?;
        assert!(store.transition_task("task", 1, "Done", false).is_err());
        store.transition_task("task", 1, "Ready", false)?;
        assert!(store.transition_task("task", 1, "Doing", false).is_err());
        store.transition_task("task", 2, "Doing", false)?;
        store.transition_task("task", 3, "Done", false)?;
        assert!(store.transition_task("task", 4, "Ready", false).is_err());
        store.transition_task("task", 4, "Ready", true)?;
        store.save("note", 1, b"source v2", None)?;
        assert!(store
            .create_task("stale", "bad", None, Some(("note", 1)))
            .is_err());
        assert!(store
            .create_task("bad-project", "bad", Some("missing"), None)
            .is_err());
        drop(store);
        let store = Store::open(directory.path().to_owned())?;
        let tasks = store.list_tasks()?;
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, "Ready");
        assert_eq!(tasks[0].row_version, 5);
        assert_eq!(tasks[0].source_revision, Some(1));
        assert_eq!(store.read("note", 1)?, b"source v1");
        assert_eq!(store.list_projects()?.len(), 1);
        Ok(())
    }
}
