//! Isolated validation workspace. No shell or external network capabilities.
use nexus_knowledge_store_probe::Store;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::Manager;

struct Workspace {
    store: Mutex<Store>,
}

fn open_workspace(root: std::path::PathBuf) -> Result<Workspace, String> {
    let mut store = Store::open(root).map_err(|e| e.to_string())?;
    store
        .ensure_note(
            "desktop-note",
            "# Nexus 桌面验证\n\n仅用于合成资料。".as_bytes(),
        )
        .map_err(|e| e.to_string())?;
    Ok(Workspace {
        store: Mutex::new(store),
    })
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
            // Separate validation identifier keeps this workspace apart from future production data.
            let root = app.path().app_data_dir()?.join("synthetic-workspace");
            let workspace = open_workspace(root).map_err(std::io::Error::other)?;
            app.manage(Arc::new(workspace));
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
        };
        assert_eq!(read(&workspace, None).unwrap().revision, 2);
        assert_eq!(read(&workspace, None).unwrap().text, "中文更新");
        assert_eq!(read(&workspace, Some(1)).unwrap().text, "original");
        assert!(read(&workspace, Some(99)).is_err());
    }

    #[test]
    fn reopening_workspace_never_reseeds_over_existing_content() {
        let directory = tempfile::tempdir().unwrap();
        let workspace = open_workspace(directory.path().to_owned()).unwrap();
        workspace
            .store
            .lock()
            .unwrap()
            .save("desktop-note", 1, b"saved draft", None)
            .unwrap();
        drop(workspace);
        let reopened = open_workspace(directory.path().to_owned()).unwrap();
        assert_eq!(read(&reopened, None).unwrap().revision, 2);
        assert_eq!(read(&reopened, None).unwrap().text, "saved draft");
        assert_eq!(read(&reopened, Some(1)).unwrap().revision, 1);
    }
}
