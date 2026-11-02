//! Storage experiment, not the production API. Immutable blobs precede DB references.
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::{error::Error, fs, path::PathBuf};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub struct Store {
    root: PathBuf,
    db: Connection,
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

impl Store {
    pub fn open(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root)?;
        let mut db = Connection::open(root.join("knowledge.db"))?;
        db.execute_batch("PRAGMA foreign_keys=ON;")?;
        let schema: i64 = db.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if schema > 1 {
            return Err("workspace schema is newer than this application; refusing writes".into());
        }
        fs::create_dir_all(root.join("blobs"))?;
        if schema == 0 {
            let tables: i64 = db.query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
                [], |row| row.get(0),
            )?;
            if tables != 0 {
                for (table, expected) in [
                    ("item", vec!["id", "revision"]),
                    (
                        "version",
                        vec!["item_id", "revision", "body_hash", "attachment_hash"],
                    ),
                ] {
                    let mut statement = db.prepare(&format!("PRAGMA table_info({table})"))?;
                    let columns = statement
                        .query_map([], |row| row.get::<_, String>(1))?
                        .collect::<std::result::Result<Vec<_>, _>>()?;
                    if tables != 2 || columns != expected {
                        return Err("unrecognized unversioned workspace; refusing migration".into());
                    }
                }
            }
            let tx = db.transaction()?;
            tx.execute_batch(
                "
             CREATE TABLE IF NOT EXISTS item(id TEXT PRIMARY KEY, revision INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS version(
               item_id TEXT NOT NULL REFERENCES item(id), revision INTEGER NOT NULL,
               body_hash TEXT NOT NULL, attachment_hash TEXT,
               PRIMARY KEY(item_id,revision));
             PRAGMA user_version=1;",
            )?;
            tx.commit()?;
        }
        Ok(Self { root, db })
    }

    fn blob(&self, bytes: &[u8]) -> Result<String> {
        let digest = hash(bytes);
        let path = self.root.join("blobs").join(&digest);
        if path.exists() {
            if hash(&fs::read(path)?) != digest {
                return Err("existing blob is corrupt".into());
            }
            return Ok(digest);
        }
        // Single-writer probe. Production must specify durable directory sync and concurrency.
        let pending = self.root.join("blobs").join(format!("{digest}.pending"));
        let file = fs::File::create(&pending)?;
        use std::io::Write;
        (&file).write_all(bytes)?;
        file.sync_all()?;
        fs::rename(pending, path)?;
        Ok(digest)
    }

    pub fn save(
        &mut self,
        id: &str,
        expected: i64,
        body: &[u8],
        attachment: Option<&[u8]>,
    ) -> Result<i64> {
        if id.trim().is_empty() || expected < 0 || expected == i64::MAX {
            return Err("invalid object id or expected revision".into());
        }
        let body_hash = self.blob(body)?;
        let attachment_hash = attachment.map(|bytes| self.blob(bytes)).transpose()?;
        let tx = self.db.transaction()?;
        tx.execute("INSERT OR IGNORE INTO item VALUES (?1,0)", [id])?;
        let changed = tx.execute(
            "UPDATE item SET revision=revision+1 WHERE id=?1 AND revision=?2",
            params![id, expected],
        )?;
        if changed != 1 {
            return Err("version conflict".into());
        }
        tx.execute(
            "INSERT INTO version VALUES (?1,?2,?3,?4)",
            params![id, expected + 1, body_hash, attachment_hash],
        )?;
        tx.commit()?;
        Ok(expected + 1)
    }

    pub fn current_revision(&self, id: &str) -> Result<i64> {
        Ok(self
            .db
            .query_row("SELECT revision FROM item WHERE id=?1", [id], |row| {
                row.get(0)
            })?)
    }

    /// Seed a synthetic workspace without treating corruption as a missing note.
    pub fn ensure_note(&mut self, id: &str, body: &[u8]) -> Result<()> {
        match self.current_revision(id) {
            Ok(_) => Ok(()),
            Err(error) => match error.downcast_ref::<rusqlite::Error>() {
                Some(rusqlite::Error::QueryReturnedNoRows) => {
                    self.save(id, 0, body, None).map(|_| ())
                }
                _ => Err(error),
            },
        }
    }

    pub fn read(&self, id: &str, revision: i64) -> Result<Vec<u8>> {
        let (body, attachment): (String, Option<String>) = self.db.query_row(
            "SELECT body_hash,attachment_hash FROM version WHERE item_id=?1 AND revision=?2",
            params![id, revision],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let content = self.verified_blob(&body)?;
        if let Some(digest) = attachment {
            self.verified_blob(&digest)?;
        }
        Ok(content)
    }

    fn verified_blob(&self, digest: &str) -> Result<Vec<u8>> {
        if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("invalid digest".into());
        }
        let bytes = fs::read(self.root.join("blobs").join(digest))?;
        if hash(&bytes) != digest {
            return Err("blob integrity failure".into());
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_reopen_and_closed_workspace_copy_preserve_content() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let mut store = Store::open(dir.path().to_owned())?;
        store.save("note", 0, b"v1", Some(b"datasheet"))?;
        store.save("note", 1, b"v2", Some(b"datasheet"))?;
        drop(store);
        let restored = tempfile::tempdir()?;
        fs::create_dir(restored.path().join("blobs"))?;
        fs::copy(
            dir.path().join("knowledge.db"),
            restored.path().join("knowledge.db"),
        )?;
        for entry in fs::read_dir(dir.path().join("blobs"))? {
            let entry = entry?;
            fs::copy(
                entry.path(),
                restored.path().join("blobs").join(entry.file_name()),
            )?;
        }
        let store = Store::open(restored.path().to_owned())?;
        assert_eq!(store.read("note", 1)?, b"v1");
        assert_eq!(store.read("note", 2)?, b"v2");
        Ok(())
    }

    #[test]
    fn rejected_commit_and_unreferenced_blob_do_not_change_current_content() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let mut store = Store::open(dir.path().to_owned())?;
        store.save("note", 0, b"v1", None)?;
        assert!(store.save("note", 0, b"stale", None).is_err());
        store.blob(b"written-before-interruption")?;
        drop(store);
        let store = Store::open(dir.path().to_owned())?;
        assert_eq!(store.read("note", 1)?, b"v1");
        assert!(store.read("note", 2).is_err());
        Ok(())
    }

    #[test]
    fn missing_attachment_and_modified_body_are_detected() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let mut store = Store::open(dir.path().to_owned())?;
        store.save("note", 0, b"body", Some(b"attachment"))?;
        fs::remove_file(dir.path().join("blobs").join(hash(b"attachment")))?;
        assert!(store.read("note", 1).is_err());
        store.blob(b"attachment")?;
        fs::write(dir.path().join("blobs").join(hash(b"body")), b"tampered")?;
        assert!(store.read("note", 1).is_err());
        Ok(())
    }

    #[test]
    fn failed_version_insert_rolls_back_revision_and_preserves_old_content() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let mut store = Store::open(dir.path().to_owned())?;
        store.save("note", 0, b"stable", None)?;
        store.db.execute_batch(
            "CREATE TRIGGER reject_version BEFORE INSERT ON version
             BEGIN SELECT RAISE(ABORT, 'injected insert failure'); END;",
        )?;
        assert!(store.save("note", 1, b"uncommitted", None).is_err());
        let revision: i64 =
            store
                .db
                .query_row("SELECT revision FROM item WHERE id='note'", [], |row| {
                    row.get(0)
                })?;
        assert_eq!(revision, 1);
        assert_eq!(store.read("note", 1)?, b"stable");
        assert!(store.read("note", 2).is_err());
        store.db.execute_batch("DROP TRIGGER reject_version")?;
        assert_eq!(store.save("note", 1, b"retry", None)?, 2);
        Ok(())
    }

    #[test]
    fn failed_blob_write_does_not_create_database_reference() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let mut store = Store::open(dir.path().to_owned())?;
        let digest = hash(b"blocked");
        // A directory at the pending-file path deterministically refuses file creation.
        fs::create_dir(dir.path().join("blobs").join(format!("{digest}.pending")))?;
        assert!(store.save("new-note", 0, b"blocked", None).is_err());
        let count: i64 = store
            .db
            .query_row("SELECT count(*) FROM item", [], |row| row.get(0))?;
        assert_eq!(count, 0);
        Ok(())
    }

    #[test]
    fn invalid_identity_or_revision_is_rejected_before_file_writes() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let mut store = Store::open(dir.path().to_owned())?;
        for (id, revision) in [(" ", 0), ("note", -1), ("note", i64::MAX)] {
            assert!(store.save(id, revision, b"must-not-write", None).is_err());
        }
        assert_eq!(fs::read_dir(dir.path().join("blobs"))?.count(), 0);
        let count: i64 = store
            .db
            .query_row("SELECT count(*) FROM item", [], |row| row.get(0))?;
        assert_eq!(count, 0);
        Ok(())
    }

    #[test]
    fn bundled_fts5_trigram_matches_chinese_and_component_models() -> Result<()> {
        let db = Connection::open_in_memory()?;
        db.execute_batch("CREATE VIRTUAL TABLE search USING fts5(body, tokenize='trigram');")?;
        db.execute(
            "INSERT INTO search(body) VALUES (?1)",
            ["双冗余电源 使用 AD5422 进行评估"],
        )?;
        for query in ["冗余电源", "AD5422"] {
            let count: i64 = db.query_row(
                "SELECT count(*) FROM search WHERE search MATCH ?1",
                [query],
                |row| row.get(0),
            )?;
            assert_eq!(count, 1, "missing query: {query}");
        }
        let short_count: i64 = db.query_row(
            "SELECT count(*) FROM search WHERE search MATCH ?1",
            ["电源"],
            |row| row.get(0),
        )?;
        // Trigram MATCH does not cover short Chinese terms; production needs a fallback.
        assert_eq!(short_count, 0);
        let fallback: i64 = db.query_row(
            "SELECT count(*) FROM search WHERE instr(body, ?1)>0",
            ["电源"],
            |row| row.get(0),
        )?;
        assert_eq!(fallback, 1);
        Ok(())
    }

    #[test]
    fn newer_workspace_is_refused_without_changing_its_schema_or_content() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let db = Connection::open(dir.path().join("knowledge.db"))?;
        db.execute_batch("PRAGMA user_version=99; CREATE TABLE future_data(value TEXT); INSERT INTO future_data VALUES ('keep');")?;
        assert!(Store::open(dir.path().to_owned()).is_err());
        let version: i64 = db.pragma_query_value(None, "user_version", |row| row.get(0))?;
        assert_eq!(version, 99);
        let value: String = db.query_row("SELECT value FROM future_data", [], |row| row.get(0))?;
        assert_eq!(value, "keep");
        let tables: i64 = db.query_row(
            "SELECT count(*) FROM sqlite_master WHERE name='item'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(tables, 0);
        Ok(())
    }

    #[test]
    fn unrelated_unversioned_database_is_not_adopted_as_a_workspace() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let db = Connection::open(dir.path().join("knowledge.db"))?;
        db.execute_batch(
            "CREATE TABLE unrelated(value TEXT); INSERT INTO unrelated VALUES ('keep');",
        )?;
        assert!(Store::open(dir.path().to_owned()).is_err());
        let version: i64 = db.pragma_query_value(None, "user_version", |row| row.get(0))?;
        assert_eq!(version, 0);
        let value: String = db.query_row("SELECT value FROM unrelated", [], |row| row.get(0))?;
        assert_eq!(value, "keep");
        Ok(())
    }
}
