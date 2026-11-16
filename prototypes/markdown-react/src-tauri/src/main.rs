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
    archived: bool,
}

#[derive(Deserialize)]
struct SaveRequest {
    id: Option<String>,
    expected_revision: i64,
    text: String,
}

#[tauri::command]
async fn read_note(
    workspace: tauri::State<'_, Arc<Workspace>>,
    id: Option<String>,
    revision: Option<i64>,
) -> Result<Note, String> {
    let workspace = workspace.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        read_by_id(
            &workspace,
            id.as_deref().unwrap_or("desktop-note"),
            revision,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

fn read_by_id(workspace: &Workspace, id: &str, revision: Option<i64>) -> Result<Note, String> {
    let store = workspace.store.lock().map_err(|_| "存储状态不可用")?;
    let revision = revision
        .map(Ok)
        .unwrap_or_else(|| store.current_revision(id))
        .map_err(|e| e.to_string())?;
    let text = String::from_utf8(store.read(id, revision).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let archived = store.is_note_archived(id).map_err(|e| e.to_string())?;
    Ok(Note {
        revision,
        text,
        archived,
    })
}

#[derive(Serialize)]
struct NoteSummary {
    id: String,
    revision: i64,
    title: String,
}

#[tauri::command]
async fn list_notes(
    workspace: tauri::State<'_, Arc<Workspace>>,
    archived: Option<bool>,
) -> Result<Vec<NoteSummary>, String> {
    let workspace = workspace.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let store = workspace.store.lock().map_err(|_| "存储状态不可用")?;
        Ok(store
            .list_notes_by_archive(archived.unwrap_or(false))
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|(id, revision, title)| NoteSummary {
                id,
                revision,
                title,
            })
            .collect())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn set_note_archived(
    workspace: tauri::State<'_, Arc<Workspace>>,
    id: String,
    expected_revision: i64,
    archived: bool,
) -> Result<i64, String> {
    let workspace = workspace.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        workspace
            .store
            .lock()
            .map_err(|_| "存储状态不可用")?
            .set_note_archived(&id, expected_revision, archived)
            .map_err(|e| e.to_string())
    })
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
                request.id.as_deref().unwrap_or("desktop-note"),
                request.expected_revision,
                request.text.as_bytes(),
                None,
            )
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn create_project(
    workspace: tauri::State<'_, Arc<Workspace>>,
    id: String,
    name: String,
) -> Result<(), String> {
    let workspace = workspace.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        workspace
            .store
            .lock()
            .map_err(|_| "存储状态不可用")?
            .create_project(&id, &name)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn list_projects(
    workspace: tauri::State<'_, Arc<Workspace>>,
) -> Result<Vec<nexus_knowledge_store_probe::tasks::Project>, String> {
    let workspace = workspace.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        workspace
            .store
            .lock()
            .map_err(|_| "存储状态不可用")?
            .list_projects()
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[derive(Deserialize)]
struct CreateTaskRequest {
    parent_id: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    id: String,
    title: String,
    project_id: Option<String>,
    source_note_id: Option<String>,
    source_revision: Option<i64>,
}
#[tauri::command]
async fn create_task(
    workspace: tauri::State<'_, Arc<Workspace>>,
    request: CreateTaskRequest,
) -> Result<(), String> {
    let source = match (&request.source_note_id, request.source_revision) {
        (Some(id), Some(revision)) => Some((id.clone(), revision)),
        (None, None) => None,
        _ => return Err("来源ID和版本必须同时提供".into()),
    };
    let workspace = workspace.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        workspace
            .store
            .lock()
            .map_err(|_| "存储状态不可用")?
            .create_task(
                &request.id,
                &request.title,
                request.project_id.as_deref(),
                source.as_ref().map(|(id, r)| (id.as_str(), *r)),
                request.parent_id.as_deref(),
                &request.tags,
            )
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn list_tasks(
    workspace: tauri::State<'_, Arc<Workspace>>,
    query: Option<nexus_knowledge_store_probe::tasks::TaskQuery>,
) -> Result<Vec<nexus_knowledge_store_probe::tasks::Task>, String> {
    let workspace = workspace.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        workspace
            .store
            .lock()
            .map_err(|_| "存储状态不可用")?
            .list_tasks(query.unwrap_or_default())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn transition_task(
    workspace: tauri::State<'_, Arc<Workspace>>,
    id: String,
    expected_row_version: i64,
    target: String,
    reopen: bool,
) -> Result<(), String> {
    let workspace = workspace.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        workspace
            .store
            .lock()
            .map_err(|_| "存储状态不可用")?
            .transition_task(&id, expected_row_version, &target, reopen)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn update_task(
    workspace: tauri::State<'_, Arc<Workspace>>,
    id: String,
    expected_row_version: i64,
    fields: nexus_knowledge_store_probe::tasks::TaskFields,
    archived: bool,
) -> Result<(), String> {
    let workspace = workspace.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        workspace
            .store
            .lock()
            .map_err(|_| "存储状态不可用")?
            .update_task(&id, expected_row_version, &fields, archived)
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
        .invoke_handler(tauri::generate_handler![
            read_note,
            save_note,
            list_notes,
            set_note_archived,
            create_project,
            list_projects,
            create_task,
            list_tasks,
            transition_task,
            update_task
        ])
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
        assert_eq!(
            read_by_id(&workspace, "desktop-note", None)
                .unwrap()
                .revision,
            2
        );
        assert_eq!(
            read_by_id(&workspace, "desktop-note", None).unwrap().text,
            "中文更新"
        );
        assert_eq!(
            read_by_id(&workspace, "desktop-note", Some(1))
                .unwrap()
                .text,
            "original"
        );
        assert!(read_by_id(&workspace, "desktop-note", Some(99)).is_err());
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
        assert_eq!(
            read_by_id(&reopened, "desktop-note", None)
                .unwrap()
                .revision,
            2
        );
        assert_eq!(
            read_by_id(&reopened, "desktop-note", None).unwrap().text,
            "saved draft"
        );
        assert_eq!(
            read_by_id(&reopened, "desktop-note", Some(1))
                .unwrap()
                .revision,
            1
        );
    }
}
