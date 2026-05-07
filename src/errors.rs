use rouille::Response;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),
    #[error("File too large: {0} bytes (max: {1} bytes)")]
    FileTooLarge(usize, usize),
    #[error("Invalid filename: {0}")]
    InvalidFilename(String),
    #[error("File content does not match extension")]
    ContentMismatch,
    #[error("Empty file")]
    EmptyFile,
}

#[derive(Error, Debug)]
pub enum ConversionError {
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),
    #[error("LibreOffice error: {0}")]
    LibreOffice(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Temporary file error: {0}")]
    TempFile(String),
    #[error("Internal server error: {0}")]
    Internal(String),
}

impl ConversionError {
    pub fn to_response(&self) -> Response {
        let (status_code, error_message) = match self {
            ConversionError::Validation(e) => (400, format!("Validation failed: {}", e)),
            ConversionError::LibreOffice(e) => (500, format!("Document conversion failed: {}", e)),
            ConversionError::Io(e) => (500, format!("File operation failed: {}", e)),
            ConversionError::TempFile(e) => (500, format!("Temporary file error: {}", e)),
            ConversionError::Internal(e) => (500, format!("Internal server error: {}", e)),
        };

        let error_response = serde_json::json!({
            "error": error_message,
            "status": status_code
        });

        Response::json(&error_response)
            .with_status_code(status_code)
    }
}

pub type Result<T> = std::result::Result<T, ConversionError>;

pub fn log_error(error: &ConversionError, context: &str) {
    tracing::error!(
        error = %error,
        context = context,
        "Conversion error occurred"
    );
}
