//! Disposable desktop adapter. No shell, external network, or personal workspace access.
use nexus_knowledge_store_probe::Store;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::Manager;

struct Workspace {
    store: Mutex<Store>,
    _directory: tempfile::TempDir,
}

#[derive(Serialize)]
struct Note {
    revision: i64,
    text: String,
}

#[derive(Deserialize)]
struct SaveRequest {
    expected_revision: i64,
    text: String,
}

fn read(workspace: &Workspace, revision: Option<i64>) -> Result<Note, String> {
    let store = workspace.store.lock().map_err(|_| "存储状态不可用")?;
    let revision = match revision {
        Some(value) => value,
        None => store
            .current_revision("desktop-note")
            .map_err(|e| e.to_string())?,
    };
    let text = String::from_utf8(
        store
            .read("desktop-note", revision)
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(Note { revision, text })
}

#[tauri::command]
async fn read_note(
    workspace: tauri::State<'_, Arc<Workspace>>,
    revision: Option<i64>,
) -> Result<Note, String> {
    let workspace = workspace.inner().clone();
    tauri::async_runtime::spawn_blocking(move || read(&workspace, revision))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn save_note(
    workspace: tauri::State<'_, Arc<Workspace>>,
    request: SaveRequest,
) -> Result<i64, String> {
    if request.text.len() > 1024 * 1024 {
        return Err("正文超过小样的 1 MiB 限制".into());
    }
    let workspace = workspace.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        workspace
            .store
            .lock()
            .map_err(|_| "存储状态不可用")?
            .save(
                "desktop-note",
                request.expected_revision,
                request.text.as_bytes(),
                None,
            )
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let directory = tempfile::tempdir()?;
            let mut store = Store::open(directory.path().to_owned())
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            store
                .save(
                    "desktop-note",
                    0,
                    "# Nexus 桌面验证\n\n仅用于合成资料。".as_bytes(),
                    None,
                )
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            app.manage(Arc::new(Workspace {
                store: Mutex::new(store),
                _directory: directory,
            }));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![read_note, save_note])
        .run(tauri::generate_context!())
        .expect("desktop probe failed");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_read_boundary_preserves_current_and_historical_versions() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = Store::open(directory.path().to_owned()).unwrap();
        store.save("desktop-note", 0, b"original", None).unwrap();
        store
            .save("desktop-note", 1, "中文更新".as_bytes(), None)
            .unwrap();
        let workspace = Workspace {
            store: Mutex::new(store),
            _directory: directory,
        };
        assert_eq!(read(&workspace, None).unwrap().revision, 2);
        assert_eq!(read(&workspace, None).unwrap().text, "中文更新");
        assert_eq!(read(&workspace, Some(1)).unwrap().text, "original");
        assert!(read(&workspace, Some(99)).is_err());
    }
}
