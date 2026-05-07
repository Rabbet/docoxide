use crate::errors::{ConversionError, Result};
use libreoffice_rs::{Office as OfficeKit, urls};

pub struct OfficeManager {
    office: OfficeKit,
}

impl OfficeManager {
    pub fn new(office_path: &str) -> Result<Self> {
        let office = OfficeKit::new(office_path).map_err(|e| {
            ConversionError::LibreOffice(format!("Failed to initialize LibreOffice: {}", e))
        })?;

        Ok(Self { office })
    }

    pub fn convert_to_pdf(&mut self, input_path: &str, output_path: &str) -> Result<()> {
        let start_time = std::time::Instant::now();

        let doc_url = urls::local_into_abs(input_path).map_err(|e| {
            ConversionError::LibreOffice(format!("Failed to convert path to URL: {}", e))
        })?;

        let output_url = urls::local_into_abs(output_path).map_err(|e| {
            ConversionError::LibreOffice(format!("Failed to convert output path to URL: {}", e))
        })?;

        let mut doc = self
            .office
            .document_load(doc_url)
            .map_err(|e| ConversionError::LibreOffice(format!("Failed to load document: {}", e)))?;

        if !doc.save_as(&output_url.to_string(), "pdf", None) {
            return Err(ConversionError::LibreOffice(
                "Failed to save PDF file".to_string(),
            ));
        }

        let duration = start_time.elapsed();
        tracing::info!(
            input_path = input_path,
            output_path = output_path,
            duration_ms = duration.as_millis(),
            "PDF conversion completed successfully"
        );

        Ok(())
    }

    pub fn convert_to_pdf_a(&mut self, input_path: &str, output_path: &str) -> Result<()> {
        let start_time = std::time::Instant::now();

        let doc_url = urls::local_into_abs(input_path).map_err(|e| {
            ConversionError::LibreOffice(format!("Failed to convert path to URL: {}", e))
        })?;

        let output_url = urls::local_into_abs(output_path).map_err(|e| {
            ConversionError::LibreOffice(format!("Failed to convert output path to URL: {}", e))
        })?;

        let mut doc = self
            .office
            .document_load(doc_url)
            .map_err(|e| ConversionError::LibreOffice(format!("Failed to load document: {}", e)))?;

        // PDF/A options are passed as a filter parameter string
        let pdf_options =
            "SelectPdfVersion=1;UseTaggedPDF=true;ExportNotes=true;ExportNotesPages=false";

        if !doc.save_as(&output_url.to_string(), "pdf", Some(pdf_options)) {
            return Err(ConversionError::LibreOffice(
                "Failed to save PDF/A file".to_string(),
            ));
        }

        let duration = start_time.elapsed();
        tracing::info!(
            input_path = input_path,
            output_path = output_path,
            duration_ms = duration.as_millis(),
            "PDF/A conversion completed successfully"
        );

        Ok(())
    }
}
