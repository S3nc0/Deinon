use serde::{Serialize, Serializer};

/// Errors shown to the person using the app, so each one says how to fix the problem.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("This file isn't a supported image. Open a JPEG, PNG, TIFF or WebP.")]
    Unsupported,
    #[error("This photo could not be read and may be damaged. Try opening another copy of it. ({0})")]
    Corrupt(String),
    #[error("This photo is too large to open: {width} × {height} px. Use a photo under {limit} megapixels.")]
    TooLarge { width: u32, height: u32, limit: u64 },
    #[error("This photo is too large to open. Use a photo under {limit} megapixels.")]
    TooLargeToDecode { limit: u64 },
    #[error("Could not open the file. Check that it still exists and that you can read it. ({0})")]
    Read(String),
    #[error("Open a photo first.")]
    NoPhoto,
    #[error("Choose the file through the app's own dialog, then try again.")]
    PathNotChosen,
    #[error("The 4 points don't form a valid shape. Move them back and try again.")]
    InvalidShape,
    #[error("{0}")]
    InvalidOptions(String),
    #[error("Nothing of the photo is left in the frame to crop to. Move the points so the photo is in the frame, or choose another option for empty areas.")]
    NothingToCrop,
    #[error("This would overwrite your original photo. Choose another file name.")]
    WouldOverwriteOriginal,
    #[error("That file already exists. Choose another file name.")]
    AlreadyExists,
    #[error("An export is already running. Wait for it to finish or cancel it.")]
    ExportRunning,
    #[error("Export cancelled.")]
    Cancelled,
    #[error("Could not save the file. Check that the folder is writable and has free space. ({0})")]
    Write(String),
    #[error("Something went wrong inside the app. Try again. ({0})")]
    Internal(String),
}

impl AppError {
    /// A short code, so the frontend can tell a cancel from a failure.
    fn code(&self) -> &'static str {
        match self {
            AppError::Cancelled => "cancelled",
            _ => "error",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("code", self.code())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}
