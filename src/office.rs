use libreoffice_rs::{urls, Office as OfficeKit};
use std::error::Error;

pub fn new_officekit(office_path: &str) -> OfficeKit {
    OfficeKit::new(office_path).unwrap()
}

pub fn convert_to_pdf(
    office: &mut OfficeKit,
    input_path: &str,
    output_path: &str,
) -> Result<(), Box<dyn Error>> {
    let doc_url = urls::local_into_abs(input_path)?;
    let mut doc = office.document_load(doc_url)?;

    if !doc.save_as(output_path, "pdf", None) {
        return Err("Failed to save PDF file!".into());
    }

    Ok(())
}
