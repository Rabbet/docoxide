use rouille::{post_input, try_or_400, Response};
use std::io::Write;
use std::net::IpAddr;
use std::path::Path;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::NamedTempFile;

use crate::errors::{log_error, ConversionError};
use crate::metrics::{HealthChecker, MetricsCollector};
use crate::office::OfficeManager;
use crate::validation::FileValidator;

const POST: &str = "POST";
const GET: &str = "GET";
const RECYCLE_INTERVAL: usize = 1000; // Recycle LibreOffice every 1000 requests

#[derive(Debug, Clone)]
enum ConversionType {
    Pdf,
    PdfA,
}

// Message types for worker thread communication
#[derive(Debug)]
struct ConversionRequest {
    file_content: Vec<u8>,
    filename: String,
    conversion_type: ConversionType,
    response_tx: mpsc::Sender<ConversionResult>,
}

type ConversionResult = Result<Vec<u8>, ConversionError>;

pub fn serve(office_path: String, port: u16) {
    let bind_addr = format!("0.0.0.0:{}", port);

    let validator = FileValidator::default();
    let metrics = MetricsCollector::new();
    let health_checker = HealthChecker::new(metrics.clone());

    // Create channel for worker thread communication
    let (conversion_tx, conversion_rx) = mpsc::channel::<ConversionRequest>();

    // Spawn LibreOffice worker thread
    let office_path_clone = office_path.clone();
    thread::spawn(move || {
        libreoffice_worker(office_path_clone, conversion_rx);
    });

    println!("Server running on http://{}", bind_addr);
    tracing::info!("Document conversion service started on {}", bind_addr);

    rouille::start_server(bind_addr, move |request| {
        let client_ip = get_client_ip(request);

        match (request.method(), request.url().as_str()) {
            (GET, "/health") => handle_health_check(&health_checker),
            (GET, "/metrics") => handle_metrics(&metrics),
            (POST, "/convert2/pdf") => handle_conversion(
                request,
                &conversion_tx,
                &validator,
                &metrics,
                client_ip,
                ConversionType::Pdf,
            ),
            (POST, "/convert2/pdfa") => handle_conversion(
                request,
                &conversion_tx,
                &validator,
                &metrics,
                client_ip,
                ConversionType::PdfA,
            ),
            _ => {
                tracing::warn!(
                    method = request.method(),
                    path = request.url(),
                    client_ip = %client_ip,
                    "Invalid request"
                );
                Response::text("Not found").with_status_code(404)
            }
        }
    });
}

fn handle_health_check(health_checker: &HealthChecker) -> Response {
    let health_status = health_checker.get_health_status();
    Response::json(&health_status)
}

fn handle_metrics(metrics: &Arc<MetricsCollector>) -> Response {
    let metrics_data = metrics.get_metrics();
    Response::json(&metrics_data)
}

fn handle_conversion(
    request: &rouille::Request,
    conversion_tx: &mpsc::Sender<ConversionRequest>,
    validator: &FileValidator,
    metrics: &Arc<MetricsCollector>,
    client_ip: IpAddr,
    conversion_type: ConversionType,
) -> Response {
    let start_time = Instant::now();

    let input = try_or_400!(post_input!(request, {
        file_content: rouille::input::post::BufferedFile,
        file_name: String,
    }));

    let file_content = input.file_content.data;
    let file_name = input.file_name;

    let validated_file = match validator.validate_file(&file_content, &file_name) {
        Ok(file) => file,
        Err(e) => {
            let error = ConversionError::from(e);
            log_error(&error, "File validation failed");
            return error.to_response();
        }
    };

    metrics.record_conversion_start(validated_file.file_size as u64);

    // Send request to worker thread
    let (response_tx, response_rx) = mpsc::channel();
    let conversion_request = ConversionRequest {
        file_content: file_content.clone(),
        filename: validated_file.sanitized_filename.clone(),
        conversion_type: conversion_type.clone(),
        response_tx,
    };

    if let Err(e) = conversion_tx.send(conversion_request) {
        let error = ConversionError::Internal(format!("Failed to send conversion request: {}", e));
        log_error(&error, "Worker communication failed");
        return error.to_response();
    }

    // Wait for response with timeout
    let result = match response_rx.recv_timeout(Duration::from_secs(60)) {
        Ok(conversion_result) => conversion_result.map(|pdf_data| {
            let file_stem = strip_extension(&validated_file.sanitized_filename);
            let output_filename = match conversion_type {
                ConversionType::Pdf => format!("{}.pdf", file_stem),
                ConversionType::PdfA => format!("{}_pdfa.pdf", file_stem),
            };
            Response::from_data("application/pdf", pdf_data)
                .with_content_disposition_attachment(&output_filename)
        }),
        Err(e) => Err(ConversionError::Internal(format!(
            "Conversion timeout: {}",
            e
        ))),
    };

    match result {
        Ok(response) => {
            metrics.record_conversion_success(start_time.elapsed());
            tracing::info!(
                client_ip = %client_ip,
                filename = validated_file.sanitized_filename,
                size = validated_file.file_size,
                duration_ms = start_time.elapsed().as_millis(),
                "Conversion completed successfully"
            );
            response
        }
        Err(error) => {
            metrics.record_conversion_failure(&error.to_string());
            log_error(&error, "Conversion failed");
            error.to_response()
        }
    }
}

fn libreoffice_worker(office_path: String, conversion_rx: mpsc::Receiver<ConversionRequest>) {
    let mut office_manager = None;
    let mut request_count = 0;

    tracing::info!("LibreOffice worker thread started");

    loop {
        // Initialize or recycle LibreOffice instance
        if office_manager.is_none() || request_count >= RECYCLE_INTERVAL {
            tracing::info!(
                "Initializing LibreOffice instance (request count: {})",
                request_count
            );

            match OfficeManager::new(&office_path) {
                Ok(manager) => {
                    office_manager = Some(manager);
                    request_count = 0;
                    tracing::info!("LibreOffice instance ready");
                }
                Err(e) => {
                    tracing::error!("Failed to initialize LibreOffice: {}", e);
                    // Sleep and retry
                    thread::sleep(Duration::from_secs(5));
                    continue;
                }
            }
        }

        // Wait for conversion request
        match conversion_rx.recv() {
            Ok(request) => {
                request_count += 1;

                tracing::debug!(
                    filename = request.filename,
                    conversion_type = ?request.conversion_type,
                    request_count = request_count,
                    "Processing conversion request"
                );

                let result = process_conversion_request(&mut office_manager, &request);

                if let Err(e) = request.response_tx.send(result) {
                    tracing::error!("Failed to send conversion response: {}", e);
                }
            }
            Err(e) => {
                tracing::error!("Worker channel disconnected: {}", e);
                break;
            }
        }
    }

    tracing::info!("LibreOffice worker thread stopped");
}

fn process_conversion_request(
    office_manager: &mut Option<OfficeManager>,
    request: &ConversionRequest,
) -> ConversionResult {
    let manager = office_manager
        .as_mut()
        .ok_or_else(|| ConversionError::Internal("LibreOffice not initialized".to_string()))?;

    let mut tmp_input_file = NamedTempFile::new().map_err(|e| {
        ConversionError::TempFile(format!("Failed to create input temp file: {}", e))
    })?;

    let tmp_output_file = NamedTempFile::new().map_err(|e| {
        ConversionError::TempFile(format!("Failed to create output temp file: {}", e))
    })?;

    tmp_input_file
        .write_all(&request.file_content)
        .map_err(|e| ConversionError::Io(e))?;

    let input_path = tmp_input_file
        .path()
        .to_str()
        .ok_or_else(|| ConversionError::Internal("Invalid input path".to_string()))?;

    let output_path = tmp_output_file
        .path()
        .to_str()
        .ok_or_else(|| ConversionError::Internal("Invalid output path".to_string()))?;

    // Perform conversion
    match request.conversion_type {
        ConversionType::Pdf => {
            manager.convert_to_pdf(input_path, output_path)?;
        }
        ConversionType::PdfA => {
            manager.convert_to_pdf_a(input_path, output_path)?;
        }
    }

    // Read and return the converted file
    let output_content =
        std::fs::read(tmp_output_file.path()).map_err(|e| ConversionError::Io(e))?;

    Ok(output_content)
}

fn get_client_ip(request: &rouille::Request) -> IpAddr {
    request
        .header("X-Forwarded-For")
        .and_then(|ip| ip.parse().ok())
        .or_else(|| request.header("X-Real-IP").and_then(|ip| ip.parse().ok()))
        .unwrap_or_else(|| request.remote_addr().ip())
}

fn strip_extension(filename: &str) -> &str {
    Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename)
}
