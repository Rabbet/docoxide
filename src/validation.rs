use crate::errors::ValidationError;
use mime_guess::MimeGuess;
use std::path::Path;

pub struct FileValidator {
    max_file_size: usize,
}

impl FileValidator {
    pub fn new(max_file_size: usize) -> Self {
        Self { max_file_size }
    }

    pub fn validate_file(
        &self,
        content: &[u8],
        filename: &str,
    ) -> Result<ValidatedFile, ValidationError> {
        if content.is_empty() {
            return Err(ValidationError::EmptyFile);
        }

        if content.len() > self.max_file_size {
            return Err(ValidationError::FileTooLarge(
                content.len(),
                self.max_file_size,
            ));
        }

        let sanitized_filename = self.sanitize_filename(filename)?;
        let detected_type = self.detect_content_type(content)?;
        let extension_type = self.get_mime_from_extension(&sanitized_filename)?;

        if !self.is_compatible_type(&detected_type, &extension_type) {
            return Err(ValidationError::ContentMismatch);
        }

        Ok(ValidatedFile {
            sanitized_filename,
            file_size: content.len(),
        })
    }

    fn sanitize_filename(&self, filename: &str) -> Result<String, ValidationError> {
        let sanitized = sanitize_filename::sanitize(filename);
        if sanitized.is_empty() {
            return Err(ValidationError::InvalidFilename(filename.to_string()));
        }
        Ok(sanitized)
    }

    fn detect_content_type(&self, content: &[u8]) -> Result<String, ValidationError> {
        if let Some(kind) = infer::get(content) {
            let mime = kind.mime_type();
            if self.is_supported_mime(mime) {
                return Ok(mime.to_string());
            }
        }

        Err(ValidationError::UnsupportedFileType(
            "Unknown or unsupported file type".to_string(),
        ))
    }

    fn get_mime_from_extension(&self, filename: &str) -> Result<String, ValidationError> {
        let path = Path::new(filename);
        let mime_guess = MimeGuess::from_path(path);

        if let Some(mime) = mime_guess.first() {
            let mime_str = mime.as_ref();
            if self.is_supported_mime(mime_str) {
                return Ok(mime_str.to_string());
            }
        }

        Err(ValidationError::UnsupportedFileType(format!(
            "Unsupported file extension: {}",
            filename
        )))
    }

    fn is_supported_mime(&self, mime: &str) -> bool {
        matches!(
            mime,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" |  // .xlsx
            "application/vnd.ms-excel" |                                          // .xls
            "application/msword" |                                               // .doc
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document" | // .docx
            "application/vnd.openxmlformats-officedocument.presentationml.presentation" | // .pptx
            "application/vnd.ms-powerpoint" |                                    // .ppt
            // Some tooling (e.g. python-docx) emits OOXML zips with non-standard
            // entry ordering, so `infer` reports `application/zip` instead of the
            // specific OOXML mime. Pair this with the extension check in
            // `is_compatible_type` to still reject random zips.
            "application/zip" |
            "application/octet-stream" // Generic binary
        )
    }

    fn is_compatible_type(&self, detected: &str, extension: &str) -> bool {
        detected == extension
            || (detected == "application/octet-stream" && self.is_supported_mime(extension))
            || (extension == "application/octet-stream" && self.is_supported_mime(detected))
            // OOXML files are zips; trust the extension when content-detection only
            // sees the underlying zip container.
            || (detected == "application/zip" && self.is_supported_mime(extension))
    }
}

#[derive(Debug)]
pub struct ValidatedFile {
    pub sanitized_filename: String,
    pub file_size: usize,
}

impl Default for FileValidator {
    fn default() -> Self {
        Self::new(100 * 1024 * 1024) // 100MB default
    }
}
