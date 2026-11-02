//! The commands the frontend can call, and the window events that feed it.
//!
//! The frontend never names an arbitrary file. Dialogs run here and drops arrive here, and the
//! resulting paths are remembered, so a command only touches a file the person actually chose.

use std::path::PathBuf;
use std::sync::atomic::Ordering;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, DragDropEvent, Emitter, Manager, State, WebviewWindow, Window};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

use crate::error::AppError;
use crate::export::{self, ExportOptions, ExportResult, Format};
use crate::menu::{self, UiState};
use crate::photo::{self, OpenedPhoto};
use crate::state::{lock, AppState};
use crate::warp::Quad;

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "tif", "tiff", "webp"];

fn join_error(e: tauri::Error) -> AppError {
    AppError::Internal(e.to_string())
}

/// Shows the native open dialog. Returns the chosen path, which `open_photo` will then accept.
#[tauri::command]
pub async fn choose_photo(window: WebviewWindow, state: State<'_, AppState>) -> Result<Option<String>, AppError> {
    let dialog = window
        .dialog()
        .file()
        .set_parent(&window)
        .set_title("Open photo")
        .add_filter("Photos", IMAGE_EXTENSIONS);
    let picked = tauri::async_runtime::spawn_blocking(move || dialog.blocking_pick_file())
        .await
        .map_err(join_error)?;
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
        return Ok(None);
    };
    state.remember_openable([path.clone()]);
    Ok(Some(path.to_string_lossy().into_owned()))
}

#[tauri::command]
pub async fn open_photo(path: String, state: State<'_, AppState>) -> Result<OpenedPhoto, AppError> {
    let path = PathBuf::from(path);
    state.check_openable(&path)?;
    if state.is_exporting() {
        return Err(AppError::ExportRunning);
    }
    let (photo, opened) = tauri::async_runtime::spawn_blocking(move || photo::open(&path))
        .await
        .map_err(join_error)??;
    *lock(&state.photo) = Some(photo);
    Ok(opened)
}

#[tauri::command]
pub async fn open_sample(state: State<'_, AppState>) -> Result<OpenedPhoto, AppError> {
    if state.is_exporting() {
        return Err(AppError::ExportRunning);
    }
    let (photo, opened) = tauri::async_runtime::spawn_blocking(photo::open_sample)
        .await
        .map_err(join_error)??;
    *lock(&state.photo) = Some(photo);
    Ok(opened)
}

/// Shows the native save dialog, which handles overwrite confirmation.
#[tauri::command]
pub async fn choose_export_path(
    window: WebviewWindow,
    state: State<'_, AppState>,
    file_name: String,
    format: Format,
) -> Result<Option<String>, AppError> {
    let name = format!("{}.{}", export::clean_file_name(&file_name), format.extension());
    let dialog = window
        .dialog()
        .file()
        .set_parent(&window)
        .set_title("Export photo")
        .set_file_name(name)
        .add_filter(format.label(), &[format.extension()]);
    let picked = tauri::async_runtime::spawn_blocking(move || dialog.blocking_save_file())
        .await
        .map_err(join_error)?;
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
        return Ok(None);
    };
    *lock(&state.save_target) = Some(path.clone());
    Ok(Some(path.to_string_lossy().into_owned()))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    pub src: Quad,
    pub dst: Quad,
    pub path: String,
    pub options: ExportOptions,
}

#[derive(Clone, Serialize)]
struct ExportProgress {
    fraction: f64,
}

#[tauri::command]
pub async fn export_photo(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ExportRequest,
) -> Result<ExportResult, AppError> {
    let path = PathBuf::from(&request.path);
    state.take_save_target(&path)?;
    let photo = state.current_photo()?;
    let _guard = state.begin_export()?;

    let worker_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = worker_app.state::<AppState>();
        let report = |fraction: f64| {
            let _ = worker_app.emit("export-progress", ExportProgress { fraction });
        };
        export::export_to_path(
            &photo,
            &request.src,
            &request.dst,
            &request.options,
            &path,
            &state.cancel_export,
            &report,
        )
    })
    .await
    .map_err(join_error)?
}

#[tauri::command]
pub fn cancel_export(state: State<'_, AppState>) {
    state.cancel_export.store(true, Ordering::SeqCst);
}

/// Called before opening another photo over an unexported shift. True means go ahead.
#[tauri::command]
pub async fn confirm_discard(window: WebviewWindow) -> Result<bool, AppError> {
    let parent = window.clone();
    let dialog = window
        .dialog()
        .message("Your shift has not been exported. If you continue, it will be lost.")
        .parent(&parent)
        .title("Discard your shift?")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Discard".to_string(),
            "Keep editing".to_string(),
        ));
    tauri::async_runtime::spawn_blocking(move || dialog.blocking_show())
        .await
        .map_err(join_error)
}

/// Keeps the window title and the native menu in step with the frontend.
#[tauri::command]
pub fn set_ui_state(window: WebviewWindow, state: State<'_, AppState>, ui: UiState) -> Result<(), AppError> {
    state.dirty.store(ui.dirty, Ordering::SeqCst);
    window.set_title(&ui.title).map_err(|e| AppError::Internal(e.to_string()))?;
    if let Some(handles) = lock(&state.menu).as_ref() {
        menu::apply(handles, &ui).map_err(|e| AppError::Internal(e.to_string()))?;
    }
    Ok(())
}

/// Asks first if there is an unexported shift, then quits.
pub fn quit_if_allowed(app: AppHandle) {
    if !app.state::<AppState>().dirty.load(Ordering::SeqCst) {
        app.exit(0);
        return;
    }
    ask_then(app, |app| app.exit(0));
}

fn ask_then(app: AppHandle, proceed: impl FnOnce(&AppHandle) + Send + 'static) {
    let mut builder = app
        .dialog()
        .message("Your shift has not been exported. If you quit now, it will be lost.")
        .title("Quit without exporting?")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Quit".to_string(),
            "Keep editing".to_string(),
        ));
    if let Some(window) = app.get_webview_window("main") {
        builder = builder.parent(&window);
    }
    builder.show(move |go| {
        if go {
            proceed(&app);
        }
    });
}

#[derive(Clone, Serialize)]
struct FileDropEvent {
    kind: &'static str,
    paths: Vec<String>,
}

pub fn on_window_event(window: &Window, event: &tauri::WindowEvent) {
    match event {
        tauri::WindowEvent::CloseRequested { api, .. } => {
            let app = window.app_handle().clone();
            if app.state::<AppState>().dirty.load(Ordering::SeqCst) {
                api.prevent_close();
                ask_then(app, |app| app.exit(0));
            }
        }
        tauri::WindowEvent::DragDrop(drop) => on_drag_drop(window, drop),
        _ => {}
    }
}

fn on_drag_drop(window: &Window, event: &DragDropEvent) {
    let payload = match event {
        DragDropEvent::Enter { .. } => FileDropEvent {
            kind: "enter",
            paths: Vec::new(),
        },
        DragDropEvent::Over { .. } => FileDropEvent {
            kind: "over",
            paths: Vec::new(),
        },
        DragDropEvent::Drop { paths, .. } => {
            window
                .state::<AppState>()
                .remember_openable(paths.iter().cloned());
            FileDropEvent {
                kind: "drop",
                paths: paths.iter().map(|p| p.to_string_lossy().into_owned()).collect(),
            }
        }
        DragDropEvent::Leave => FileDropEvent {
            kind: "leave",
            paths: Vec::new(),
        },
        _ => return,
    };
    let _ = window.emit("file-drop", payload);
}
