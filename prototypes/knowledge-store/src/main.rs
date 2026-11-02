//! Test-only process bridge. Production calls Store through the Tauri service boundary.
use nexus_knowledge_store_probe::Store;
use serde_json::{json, Value};
use std::{error::Error, io::Read, path::PathBuf};

fn run() -> Result<Value, Box<dyn Error>> {
    let root = std::env::args_os()
        .nth(1)
        .ok_or("missing temporary workspace")?;
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let request: Value = serde_json::from_str(&input)?;
    let id = request["id"].as_str().ok_or("missing id")?;
    let revision = request["revision"].as_i64().ok_or("missing revision")?;
    if revision < 0 {
        return Err("negative revision".into());
    }
    let mut store = Store::open(PathBuf::from(root))?;
    match request["operation"].as_str() {
        Some("read") => {
            let revision = if revision == 0 {
                store.current_revision(id)?
            } else {
                revision
            };
            Ok(json!({"revision": revision, "text": String::from_utf8(store.read(id, revision)?)?}))
        }
        Some("save") => {
            let text = request["text"].as_str().ok_or("missing text")?;
            Ok(json!({"revision": store.save(id, revision, text.as_bytes(), None)?}))
        }
        _ => Err("unknown operation".into()),
    }
}

fn main() {
    let response = match run() {
        Ok(value) => json!({"ok": true, "value": value}),
        Err(error) => json!({"ok": false, "error": error.to_string()}),
    };
    println!("{response}");
}
