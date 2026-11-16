//! Office catalog increment. State names/targets reuse Worktrace 7b9cd47.
use crate::{task_status::TaskStatus, Result, Store};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub(crate) const SCHEMA_4: &str = "
ALTER TABLE task ADD COLUMN parent_id TEXT REFERENCES task(id);
ALTER TABLE task ADD COLUMN archived INTEGER NOT NULL DEFAULT 0 CHECK(archived IN (0,1));
CREATE TABLE task_tag(task_id TEXT NOT NULL REFERENCES task(id),tag TEXT NOT NULL,PRIMARY KEY(task_id,tag));
CREATE INDEX task_parent ON task(parent_id);
CREATE INDEX tag_lookup ON task_tag(tag,task_id);
PRAGMA user_version=4;";

#[derive(Default, Deserialize)]
pub struct TaskQuery {
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub archived: bool,
}
#[derive(Deserialize)]
pub struct TaskFields {
    pub title: String,
    pub project_id: Option<String>,
    pub parent_id: Option<String>,
    pub tags: Vec<String>,
}

fn normalized_tags(tags: &[String]) -> Result<BTreeSet<String>> {
    if tags.len() > 20 {
        return Err("每个任务最多20个标签".into());
    }
    tags.iter()
        .map(|tag| {
            let tag = tag.trim().trim_start_matches('#').trim().to_lowercase();
            if tag.is_empty() || tag.chars().count() > 40 {
                return Err("标签不能为空或超过40字".into());
            }
            Ok(tag)
        })
        .collect()
}
fn check_parent(
    db: &rusqlite::Connection,
    id: &str,
    project: Option<&str>,
    parent: Option<&str>,
    archived: bool,
) -> Result<()> {
    if let Some(parent) = parent {
        crate::timer::ensure_task_not_timed(db, parent)?;
        let (parent_project, parent_archived, status): (Option<String>, bool, String) = db
            .query_row(
                "SELECT project_id,archived,status FROM task WHERE id=?1",
                [parent],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?;
        if parent_project.as_deref() != project {
            return Err("子任务必须与父任务属于同一项目".into());
        }
        if !archived
            && (parent_archived || TaskStatus::parse(&status).ok_or("未知状态")?.is_terminal())
        {
            return Err("不能挂到已归档或已结束的父任务".into());
        }
        let cycle: bool = db.query_row("WITH RECURSIVE ancestors(id) AS (SELECT ?1 UNION SELECT task.parent_id FROM task JOIN ancestors ON task.id=ancestors.id WHERE task.parent_id IS NOT NULL) SELECT EXISTS(SELECT 1 FROM ancestors WHERE id=?2)",params![parent,id],|r| r.get(0))?;
        if cycle {
            return Err("父子任务不能形成循环".into());
        }
    }
    Ok(())
}
fn write_tags(db: &rusqlite::Connection, id: &str, tags: &BTreeSet<String>) -> Result<()> {
    db.execute("DELETE FROM task_tag WHERE task_id=?1", [id])?;
    for tag in tags {
        db.execute(
            "INSERT INTO task_tag(task_id,tag) VALUES (?1,?2)",
            params![id, tag],
        )?;
    }
    Ok(())
}

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
    pub parent_id: Option<String>,
    pub archived: bool,
    pub tags: Vec<String>,
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
        parent: Option<&str>,
        tags: &[String],
    ) -> Result<()> {
        let title = text(title)?;
        let id = text(id)?;
        let tags = normalized_tags(tags)?;
        if let Some((note, revision)) = source {
            self.read(note, revision)?;
        }
        let tx = self.db.transaction()?;
        check_parent(&tx, id, project, parent, false)?;
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
        tx.execute(
            "UPDATE task SET parent_id=?2 WHERE id=?1",
            params![id, parent],
        )?;
        write_tags(&tx, id, &tags)?;
        tx.commit()?;
        Ok(())
    }
    pub fn list_tasks(&self, filter: TaskQuery) -> Result<Vec<Task>> {
        let tag = filter
            .tag
            .trim()
            .trim_start_matches('#')
            .trim()
            .to_lowercase();
        let mut query = self.db.prepare("SELECT id,title,project_id,status,row_version,source_note_id,source_revision,parent_id,archived FROM task WHERE archived=?1 AND (?2='' OR EXISTS(SELECT 1 FROM task_tag WHERE task_id=task.id AND tag=?2)) AND (?3='' OR instr(lower(title),lower(?3))>0) ORDER BY rowid DESC")?;
        let rows = query.query_map(params![filter.archived, tag, filter.text.trim()], |r| {
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
                parent_id: r.get(7)?,
                archived: r.get(8)?,
                tags: Vec::new(),
            })
        })?;
        let mut tasks = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        for task in &mut tasks {
            let mut tags = self
                .db
                .prepare("SELECT tag FROM task_tag WHERE task_id=?1 ORDER BY tag")?;
            task.tags = tags
                .query_map([&task.id], |r| r.get(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
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
    pub fn update_task(
        &mut self,
        id: &str,
        expected: i64,
        fields: &TaskFields,
        archived: bool,
    ) -> Result<()> {
        let title = text(&fields.title)?;
        let tags = normalized_tags(&fields.tags)?;
        if expected < 1 || expected == i64::MAX {
            return Err("无效版本".into());
        }
        let tx = self.db.transaction()?;
        crate::timer::ensure_task_not_timed(&tx, id)?;
        check_parent(
            &tx,
            id,
            fields.project_id.as_deref(),
            fields.parent_id.as_deref(),
            archived,
        )?;
        let incompatible_child: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM task WHERE parent_id=?1 AND (project_id IS NOT ?2 OR (?3 AND archived=0)))",params![id,fields.project_id,archived],|r|r.get(0))?;
        if incompatible_child {
            return Err("请先处理子任务，再更改父任务项目或归档父任务".into());
        }
        let changed = tx.execute("UPDATE task SET title=?3,project_id=?4,parent_id=?5,archived=?6,row_version=row_version+1 WHERE id=?1 AND row_version=?2",params![id,expected,title,fields.project_id,fields.parent_id,archived])?;
        if changed != 1 {
            return Err("任务已修改，请刷新后重试".into());
        }
        write_tags(&tx, id, &tags)?;
        tx.commit()?;
        Ok(())
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
        crate::timer::ensure_task_not_timed(&tx, id)?;
        let current: (String, i64) = tx.query_row(
            "SELECT status,row_version FROM task WHERE id=?1 AND archived=0",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if current.1 != expected {
            return Err("任务已修改，请刷新后重试".into());
        }
        let from = TaskStatus::parse(&current.0).ok_or("未知任务状态")?;
        if !target.is_terminal() {
            let (project, parent): (Option<String>, Option<String>) = tx.query_row(
                "SELECT project_id,parent_id FROM task WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            check_parent(&tx, id, project.as_deref(), parent.as_deref(), false)?;
        }
        if target.is_terminal() {
            let unfinished: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM task WHERE parent_id=?1 AND archived=0 AND status NOT IN ('Done','Cancelled'))",[id],|r|r.get(0))?;
            if unfinished {
                return Err("请先完成或取消子任务".into());
            }
        }
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
    fn child_reopen_requires_reopening_its_finished_parent() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let mut store = Store::open(directory.path().to_owned())?;
        store.create_task("p", "parent", None, None, None, &[])?;
        store.create_task("c", "child", None, None, Some("p"), &[])?;
        store.transition_task("p", 1, "Ready", false)?;
        store.transition_task("c", 1, "Ready", false)?;
        store.transition_task("c", 2, "Done", false)?;
        store.transition_task("p", 2, "Done", false)?;
        assert!(store.transition_task("c", 3, "Ready", true).is_err());
        store.transition_task("p", 3, "Ready", true)?;
        store.transition_task("c", 3, "Ready", true)?;
        Ok(())
    }
    #[test]
    fn schema_three_migration_preserves_existing_tasks_and_snapshot() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let db = rusqlite::Connection::open(directory.path().join("knowledge.db"))?;
        db.execute_batch("CREATE TABLE item(id TEXT PRIMARY KEY,revision INTEGER NOT NULL,archived INTEGER NOT NULL DEFAULT 0); CREATE TABLE version(item_id TEXT NOT NULL REFERENCES item(id),revision INTEGER NOT NULL,body_hash TEXT NOT NULL,attachment_hash TEXT,PRIMARY KEY(item_id,revision));")?;
        db.execute_batch(SCHEMA)?;
        db.execute("INSERT INTO project(id,name) VALUES ('p','existing')", [])?;
        db.execute("INSERT INTO task(id,title,project_id,status,row_version) VALUES ('t','old task','p','Ready',5)",[])?;
        drop(db);
        let store = Store::open(directory.path().to_owned())?;
        let tasks = store.list_tasks(Default::default())?;
        assert_eq!(tasks[0].row_version, 5);
        assert_eq!(tasks[0].status, "Ready");
        assert!(tasks[0].parent_id.is_none());
        assert!(tasks[0].tags.is_empty());
        let snapshot = rusqlite::Connection::open(directory.path().join("before-schema-4.db"))?;
        assert_eq!(
            snapshot.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))?,
            3
        );
        Ok(())
    }
    #[test]
    fn subtasks_tags_search_cycle_conflicts_and_archive_restore() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let mut store = Store::open(directory.path().to_owned())?;
        store.create_project("p", "project")?;
        store.create_project("q", "other")?;
        store.create_task("parent", "主任务", Some("p"), None, None, &["工程".into()])?;
        store.create_task(
            "child",
            "电源评估",
            Some("p"),
            None,
            Some("parent"),
            &[" #Hardware ".into(), "hardware".into(), "工程".into()],
        )?;
        assert!(store
            .create_task("wrong", "wrong", Some("q"), None, Some("parent"), &[])
            .is_err());
        let fields = TaskFields {
            title: "parent".into(),
            project_id: Some("p".into()),
            parent_id: Some("child".into()),
            tags: vec![],
        };
        assert!(store.update_task("parent", 1, &fields, false).is_err());
        let mut parent_fields = TaskFields {
            parent_id: None,
            ..fields
        };
        assert!(store
            .update_task("parent", 1, &parent_fields, true)
            .is_err());
        parent_fields.project_id = Some("q".into());
        assert!(store
            .update_task("parent", 1, &parent_fields, false)
            .is_err());
        store.transition_task("parent", 1, "Ready", false)?;
        assert!(store.transition_task("parent", 2, "Done", false).is_err());
        let results = store.list_tasks(TaskQuery {
            tag: "#HARDWARE".into(),
            text: "电源".into(),
            ..Default::default()
        })?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "child");
        assert_eq!(results[0].tags.len(), 2);
        let child_fields = TaskFields {
            title: "完成评估".into(),
            project_id: Some("p".into()),
            parent_id: Some("parent".into()),
            tags: vec!["验证".into()],
        };
        store.update_task("child", 1, &child_fields, false)?;
        assert!(store.update_task("child", 1, &child_fields, true).is_err());
        assert!(store
            .list_tasks(TaskQuery {
                tag: "hardware".into(),
                ..Default::default()
            })?
            .is_empty());
        store.update_task("child", 2, &child_fields, true)?;
        assert_eq!(
            store
                .list_tasks(TaskQuery {
                    tag: "验证".into(),
                    archived: true,
                    ..Default::default()
                })?
                .len(),
            1
        );
        assert!(store.transition_task("child", 3, "Ready", false).is_err());
        store.update_task("child", 3, &child_fields, false)?;
        drop(store);
        let store = Store::open(directory.path().to_owned())?;
        let results = store.list_tasks(TaskQuery {
            tag: "验证".into(),
            ..Default::default()
        })?;
        assert_eq!(results[0].parent_id.as_deref(), Some("parent"));
        assert_eq!(results[0].row_version, 4);
        Ok(())
    }
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
            .create_task(
                "t",
                "rejected archived source",
                None,
                Some(("legacy", 2)),
                None,
                &[]
            )
            .is_err());
        assert!(store.list_tasks(Default::default())?.is_empty());
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
        store.create_task(
            "task",
            "评估方案",
            Some("project"),
            Some(("note", 1)),
            None,
            &[],
        )?;
        assert!(store.transition_task("task", 1, "Done", false).is_err());
        store.transition_task("task", 1, "Ready", false)?;
        assert!(store.transition_task("task", 1, "Doing", false).is_err());
        store.transition_task("task", 2, "Doing", false)?;
        store.transition_task("task", 3, "Done", false)?;
        assert!(store.transition_task("task", 4, "Ready", false).is_err());
        store.transition_task("task", 4, "Ready", true)?;
        store.save("note", 1, b"source v2", None)?;
        assert!(store
            .create_task("stale", "bad", None, Some(("note", 1)), None, &[])
            .is_err());
        assert!(store
            .create_task("bad-project", "bad", Some("missing"), None, None, &[])
            .is_err());
        drop(store);
        let store = Store::open(directory.path().to_owned())?;
        let tasks = store.list_tasks(Default::default())?;
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, "Ready");
        assert_eq!(tasks[0].row_version, 5);
        assert_eq!(tasks[0].source_revision, Some(1));
        assert_eq!(store.read("note", 1)?, b"source v1");
        assert_eq!(store.list_projects()?.len(), 1);
        Ok(())
    }
}
