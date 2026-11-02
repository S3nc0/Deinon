//! The native menu. Items only name a command: the frontend runs the same code as the toolbar,
//! then reports back which items should be enabled or checked.

use serde::Deserialize;
#[cfg(target_os = "macos")]
use tauri::menu::PredefinedMenuItem;
use tauri::menu::{
    CheckMenuItem, CheckMenuItemBuilder, Menu, MenuBuilder, MenuItem, MenuItemBuilder,
    SubmenuBuilder,
};
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::state::AppState;

pub struct MenuHandles {
    export: MenuItem<Wry>,
    undo: MenuItem<Wry>,
    redo: MenuItem<Wry>,
    reset: MenuItem<Wry>,
    fit: MenuItem<Wry>,
    zoom_in: MenuItem<Wry>,
    zoom_out: MenuItem<Wry>,
    compare: CheckMenuItem<Wry>,
    mesh: CheckMenuItem<Wry>,
    grid: CheckMenuItem<Wry>,
}

/// What the frontend reports after each change.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiState {
    pub title: String,
    pub dirty: bool,
    pub photo_open: bool,
    pub can_undo: bool,
    pub can_redo: bool,
    pub can_compare: bool,
    pub compare: bool,
    pub mesh: bool,
    pub grid: bool,
}

pub fn build(app: &AppHandle) -> tauri::Result<(Menu<Wry>, MenuHandles)> {
    let item = |id: &str, text: &str, accel: &str, enabled: bool| {
        MenuItemBuilder::with_id(id, text)
            .accelerator(accel)
            .enabled(enabled)
            .build(app)
    };
    let check = |id: &str, text: &str, accel: &str| {
        CheckMenuItemBuilder::with_id(id, text)
            .accelerator(accel)
            .enabled(false)
            .build(app)
    };

    let open = item("open", "Open photo…", "CmdOrCtrl+O", true)?;
    let export = item("export", "Export…", "CmdOrCtrl+E", false)?;
    let quit = item("quit", "Quit", "CmdOrCtrl+Q", true)?;
    let undo = item("undo", "Undo", "CmdOrCtrl+Z", false)?;
    let redo = item("redo", "Redo", "CmdOrCtrl+Shift+Z", false)?;
    let reset = item("reset", "Reset the shift", "CmdOrCtrl+Shift+R", false)?;
    let fit = item("fit", "Fit to window", "CmdOrCtrl+0", false)?;
    let zoom_in = item("zoom-in", "Zoom in", "CmdOrCtrl+=", false)?;
    let zoom_out = item("zoom-out", "Zoom out", "CmdOrCtrl+-", false)?;
    let compare = check("compare", "Compare", "CmdOrCtrl+Shift+C")?;
    let mesh = check("mesh", "Perspective mesh", "CmdOrCtrl+Shift+M")?;
    let grid = check("grid", "Thirds grid", "CmdOrCtrl+Shift+G")?;
    let theme = item("theme", "Switch theme", "CmdOrCtrl+Shift+T", true)?;

    let file = SubmenuBuilder::new(app, "File")
        .item(&open)
        .item(&export)
        .separator()
        .item(&quit)
        .build()?;

    #[cfg_attr(not(target_os = "macos"), allow(unused_mut))]
    let mut edit = SubmenuBuilder::new(app, "Edit").item(&undo).item(&redo).item(&reset);
    // A webview on macOS only gets copy and paste through these menu items.
    #[cfg(target_os = "macos")]
    {
        edit = edit
            .separator()
            .item(&PredefinedMenuItem::cut(app, None)?)
            .item(&PredefinedMenuItem::copy(app, None)?)
            .item(&PredefinedMenuItem::paste(app, None)?)
            .item(&PredefinedMenuItem::select_all(app, None)?);
    }
    let edit = edit.build()?;

    let view = SubmenuBuilder::new(app, "View")
        .item(&fit)
        .item(&zoom_in)
        .item(&zoom_out)
        .separator()
        .item(&compare)
        .item(&mesh)
        .item(&grid)
        .separator()
        .item(&theme)
        .build()?;

    #[cfg_attr(not(target_os = "macos"), allow(unused_mut))]
    let mut menu = MenuBuilder::new(app);
    // macOS turns the first menu into the app menu, whatever it is called.
    #[cfg(target_os = "macos")]
    {
        let app_menu = SubmenuBuilder::new(app, "FPV Shifter")
            .item(&PredefinedMenuItem::about(app, None, None)?)
            .separator()
            .item(&PredefinedMenuItem::services(app, None)?)
            .separator()
            .item(&PredefinedMenuItem::hide(app, None)?)
            .item(&PredefinedMenuItem::hide_others(app, None)?)
            .item(&PredefinedMenuItem::show_all(app, None)?)
            .build()?;
        menu = menu.item(&app_menu);
    }
    let menu = menu.item(&file).item(&edit).item(&view).build()?;

    Ok((
        menu,
        MenuHandles {
            export,
            undo,
            redo,
            reset,
            fit,
            zoom_in,
            zoom_out,
            compare,
            mesh,
            grid,
        },
    ))
}

pub fn apply(handles: &MenuHandles, ui: &UiState) -> tauri::Result<()> {
    handles.export.set_enabled(ui.photo_open)?;
    handles.undo.set_enabled(ui.can_undo)?;
    handles.redo.set_enabled(ui.can_redo)?;
    handles.reset.set_enabled(ui.photo_open)?;
    handles.fit.set_enabled(ui.photo_open)?;
    handles.zoom_in.set_enabled(ui.photo_open)?;
    handles.zoom_out.set_enabled(ui.photo_open)?;
    handles.compare.set_enabled(ui.can_compare)?;
    handles.compare.set_checked(ui.compare)?;
    handles.mesh.set_enabled(ui.photo_open)?;
    handles.mesh.set_checked(ui.mesh)?;
    handles.grid.set_enabled(ui.photo_open)?;
    handles.grid.set_checked(ui.grid)?;
    Ok(())
}

pub fn on_event(app: &AppHandle, id: &str) {
    if id == "quit" {
        let app = app.clone();
        crate::commands::quit_if_allowed(app);
    } else {
        let _ = app.emit("menu-command", id);
    }
}

/// Keeps the handles in state so `set_ui_state` can update them.
pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let (menu, handles) = build(app)?;
    app.set_menu(menu)?;
    let state = app.state::<AppState>();
    if let Ok(mut slot) = state.menu.lock() {
        *slot = Some(handles);
    }
    app.on_menu_event(|app, event| on_event(app, event.id().as_ref()));
    Ok(())
}
