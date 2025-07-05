use rouille::{post_input, try_or_400, Response};
use std::io::{Read, Write};
use std::path::Path;
use tempfile::NamedTempFile;

use crate::office;

const POST: &str = "POST";

pub fn serve(office_path: String, port: u16) {
    let bind_addr = format!("0.0.0.0:{}", port);

    println!("Server running on http://{}", bind_addr);

    rouille::start_server(bind_addr, move |request| {
        if request.method() != POST {
            tracing::warn!(
                method = request.method(),
                path = request.url(),
                "Invalid request"
            );
            return Response::text("Method not allowed").with_status_code(405);
        }

        match request.url().as_str() {
            "/convert/to-pdf" => {
                let input = try_or_400!(post_input!(request, {
                    file_content: rouille::input::post::BufferedFile,
                    file_name: String,
                }));

                let file_content = input.file_content.data;
                let file_stem = strip_extension(input.file_name.as_str());

                // Create temporary files
                let mut tmp_input_file = NamedTempFile::new().unwrap();
                let tmp_output_file = NamedTempFile::new().unwrap();

                tmp_input_file.write_all(&file_content).unwrap();

                let mut office = office::new_officekit(&office_path);
                office::convert_to_pdf(
                    &mut office,
                    tmp_input_file.path().to_str().unwrap(),
                    tmp_output_file.path().to_str().unwrap(),
                );

                tracing::info!(
                    input_bytes = std::fs::metadata(tmp_input_file.path()).unwrap().len(),
                    output_bytes = std::fs::metadata(tmp_output_file.path()).unwrap().len(),
                    "File sizes"
                );

                let (file, _path) = tmp_output_file.keep().unwrap();
                let len = file.metadata().unwrap().len();

                tracing::info!("File: {:#?}", file);
                tracing::info!("File len: {:#?}", len);

                Response::from_file("application/pdf", file)
                    .with_content_disposition_attachment(&format!("{}.pdf", file_stem))
            }
            /*
                        "/convert/to-pdfa" => {
                            let input = try_or_400!(receive_upload(&request));
                            let file_content = input.file_content;
                            let file_stem = strip_extension(input.file_name);

                            // Create temporary files
                            let tmp_input_file = NamedTempFile::new().unwrap();
                            let tmp_output_file = NamedTempFile::new().unwrap();

                            tmp_input_file.write_all(&file_content).unwrap();

                            let mut office = office::new_officekit(office_path);
                            office::convert_to_pdfa(
                                &mut office,
                                tmp_input_file.path().to_str().unwrap(),
                                tmp_output_file.path().to_str().unwrap(),
                            );

                            Response::from_file("application/pdf", tmp_output_file.into_file())
                                .with_content_disposition_attachment(&format!(
                                    "attachment; filename=\"{}.pdf\"",
                                    file_stem
                                ))
                        }
            */
            _ => Response::empty_404(),
        }
    });
}

fn strip_extension(filename: &str) -> &str {
    Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename)
}
