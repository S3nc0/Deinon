use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use crate::error::AppError;
use crate::menu::MenuHandles;
use crate::photo::Photo;

#[derive(Default)]
pub struct AppState {
    /// The open photo, at full resolution.
    pub photo: Mutex<Option<Arc<Photo>>>,
    /// Paths the person chose through a dialog or a drop. Commands only open these.
    pub openable: Mutex<HashSet<PathBuf>>,
    /// The path the person chose in the save dialog. Used once.
    pub save_target: Mutex<Option<PathBuf>>,
    pub cancel_export: AtomicBool,
    pub exporting: AtomicBool,
    /// The current shift has not been exported.
    pub dirty: AtomicBool,
    pub menu: Mutex<Option<MenuHandles>>,
}

/// A poisoned lock only means a thread panicked while holding it, and the data here stays usable.
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl AppState {
    pub fn remember_openable(&self, paths: impl IntoIterator<Item = PathBuf>) {
        lock(&self.openable).extend(paths);
    }

    pub fn check_openable(&self, path: &PathBuf) -> Result<(), AppError> {
        if lock(&self.openable).contains(path) {
            Ok(())
        } else {
            Err(AppError::PathNotChosen)
        }
    }

    pub fn take_save_target(&self, path: &PathBuf) -> Result<(), AppError> {
        let mut slot = lock(&self.save_target);
        if slot.as_ref() == Some(path) {
            *slot = None;
            Ok(())
        } else {
            Err(AppError::PathNotChosen)
        }
    }

    pub fn current_photo(&self) -> Result<Arc<Photo>, AppError> {
        lock(&self.photo).clone().ok_or(AppError::NoPhoto)
    }

    pub fn is_exporting(&self) -> bool {
        self.exporting.load(Ordering::SeqCst)
    }

    /// Marks an export as running until the guard drops, even if the export fails or panics.
    pub fn begin_export(&self) -> Result<ExportGuard<'_>, AppError> {
        if self.exporting.swap(true, Ordering::SeqCst) {
            return Err(AppError::ExportRunning);
        }
        self.cancel_export.store(false, Ordering::SeqCst);
        Ok(ExportGuard(self))
    }
}

pub struct ExportGuard<'a>(&'a AppState);

impl Drop for ExportGuard<'_> {
    fn drop(&mut self) {
        self.0.exporting.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_chosen_paths_can_be_opened() {
        let state = AppState::default();
        let chosen = PathBuf::from("/photos/a.jpg");
        assert!(matches!(state.check_openable(&chosen), Err(AppError::PathNotChosen)));
        state.remember_openable([chosen.clone()]);
        assert!(state.check_openable(&chosen).is_ok());
        assert!(state.check_openable(&PathBuf::from("/etc/passwd")).is_err());
    }

    #[test]
    fn a_save_target_works_once_and_only_for_its_own_path() {
        let state = AppState::default();
        let target = PathBuf::from("/photos/out.jpg");
        *lock(&state.save_target) = Some(target.clone());
        assert!(state.take_save_target(&PathBuf::from("/photos/other.jpg")).is_err());
        assert!(state.take_save_target(&target).is_ok());
        assert!(state.take_save_target(&target).is_err());
    }

    #[test]
    fn only_one_export_runs_at_a_time() {
        let state = AppState::default();
        let guard = state.begin_export().unwrap();
        assert!(matches!(state.begin_export(), Err(AppError::ExportRunning)));
        assert!(state.is_exporting());
        drop(guard);
        assert!(!state.is_exporting());
        assert!(state.begin_export().is_ok());
    }
}
