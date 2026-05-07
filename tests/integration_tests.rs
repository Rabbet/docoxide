use docoxide::http_server;
use reqwest::multipart;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

const TEST_SERVER_PORT: u16 = 8765;
const TEST_SERVER_URL: &str = "http://localhost:8765";

static SERVER_STARTED: OnceLock<()> = OnceLock::new();

fn ensure_server_running() {
    SERVER_STARTED.get_or_init(|| {
        thread::spawn(|| {
            http_server::serve("/usr/lib/libreoffice/program".to_string(), TEST_SERVER_PORT);
        });
    });
}

async fn wait_for_server_ready() {
    let client = reqwest::Client::new();
    let mut attempts = 0;

    while attempts < 30 {
        if let Ok(response) = client
            .get(format!("{}/health", TEST_SERVER_URL))
            .send()
            .await
            && response.status().is_success()
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
        attempts += 1;
    }

    panic!("Server failed to start within timeout");
}

fn get_test_file_path(filename: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(filename)
        .to_string_lossy()
        .to_string()
}

#[tokio::test]
async fn test_health_endpoint() {
    ensure_server_running();
    wait_for_server_ready().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/health", TEST_SERVER_URL))
        .send()
        .await
        .expect("Failed to make health request");

    assert_eq!(response.status(), 200);

    let health_data: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse health response");

    assert!(health_data["status"].is_string());
    assert!(health_data["version"].is_string());
    assert!(health_data["uptime_seconds"].is_number());
    assert!(health_data["metrics"].is_object());
}

#[tokio::test]
async fn test_metrics_endpoint() {
    ensure_server_running();
    wait_for_server_ready().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/metrics", TEST_SERVER_URL))
        .send()
        .await
        .expect("Failed to make metrics request");

    assert_eq!(response.status(), 200);

    let metrics_data: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse metrics response");

    assert!(metrics_data["total_conversions"].is_number());
    assert!(metrics_data["successful_conversions"].is_number());
    assert!(metrics_data["failed_conversions"].is_number());
    assert!(metrics_data["total_bytes_processed"].is_number());
}

#[tokio::test]
async fn test_successful_docx_conversion() {
    ensure_server_running();
    wait_for_server_ready().await;
    let client = reqwest::Client::new();

    let file_path = get_test_file_path("test_document.docx");
    let file_content = fs::read(&file_path).expect("Failed to read test document");

    let form = multipart::Form::new()
        .part(
            "file_content",
            multipart::Part::bytes(file_content)
                .mime_str("application/octet-stream")
                .unwrap()
                .file_name("test_document.docx"),
        )
        .text("file_name", "test_document.docx");

    let response = client
        .post(format!("{}/convert2/pdf", TEST_SERVER_URL))
        .multipart(form)
        .send()
        .await
        .expect("Failed to make conversion request");

    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/pdf"
    );

    let pdf_content = response.bytes().await.expect("Failed to read PDF response");
    assert!(!pdf_content.is_empty());

    // Verify PDF header
    assert_eq!(&pdf_content[0..4], b"%PDF");
}

#[tokio::test]
async fn test_successful_xlsx_conversion() {
    ensure_server_running();
    wait_for_server_ready().await;
    let client = reqwest::Client::new();

    let file_path = get_test_file_path("test_spreadsheet.xlsx");
    let file_content = fs::read(&file_path).expect("Failed to read test spreadsheet");

    let form = multipart::Form::new()
        .part(
            "file_content",
            multipart::Part::bytes(file_content)
                .mime_str("application/octet-stream")
                .unwrap()
                .file_name("test_spreadsheet.xlsx"),
        )
        .text("file_name", "test_spreadsheet.xlsx");

    let response = client
        .post(format!("{}/convert2/pdf", TEST_SERVER_URL))
        .multipart(form)
        .send()
        .await
        .expect("Failed to make conversion request");

    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/pdf"
    );

    let pdf_content = response.bytes().await.expect("Failed to read PDF response");
    assert!(!pdf_content.is_empty());

    // Verify PDF header
    assert_eq!(&pdf_content[0..4], b"%PDF");
}

#[tokio::test]
async fn test_pdfa_conversion() {
    ensure_server_running();
    wait_for_server_ready().await;
    let client = reqwest::Client::new();

    let file_path = get_test_file_path("test_document.docx");
    let file_content = fs::read(&file_path).expect("Failed to read test document");

    let form = multipart::Form::new()
        .part(
            "file_content",
            multipart::Part::bytes(file_content)
                .mime_str("application/octet-stream")
                .unwrap()
                .file_name("test_document.docx"),
        )
        .text("file_name", "test_document.docx");

    let response = client
        .post(format!("{}/convert2/pdfa", TEST_SERVER_URL))
        .multipart(form)
        .send()
        .await
        .expect("Failed to make PDF/A conversion request");

    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/pdf"
    );

    let pdf_content = response.bytes().await.expect("Failed to read PDF response");
    assert!(!pdf_content.is_empty());

    // Verify PDF header
    assert_eq!(&pdf_content[0..4], b"%PDF");
}

#[tokio::test]
async fn test_empty_file_rejection() {
    ensure_server_running();
    wait_for_server_ready().await;
    let client = reqwest::Client::new();

    let file_path = get_test_file_path("empty_file.docx");
    let file_content = fs::read(&file_path).expect("Failed to read empty file");

    let form = multipart::Form::new()
        .part(
            "file_content",
            multipart::Part::bytes(file_content)
                .mime_str("application/octet-stream")
                .unwrap()
                .file_name("empty_file.docx"),
        )
        .text("file_name", "empty_file.docx");

    let response = client
        .post(format!("{}/convert2/pdf", TEST_SERVER_URL))
        .multipart(form)
        .send()
        .await
        .expect("Failed to make conversion request");

    assert_eq!(response.status(), 400);

    let error_response: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse error response");

    assert!(
        error_response["error"]
            .as_str()
            .unwrap()
            .contains("Empty file")
    );
}

#[tokio::test]
async fn test_invalid_file_type_rejection() {
    ensure_server_running();
    wait_for_server_ready().await;
    let client = reqwest::Client::new();

    let file_path = get_test_file_path("test_document.txt");
    let file_content = fs::read(&file_path).expect("Failed to read text file");

    let form = multipart::Form::new()
        .part(
            "file_content",
            multipart::Part::bytes(file_content)
                .mime_str("application/octet-stream")
                .unwrap()
                .file_name("test_document.txt"),
        )
        .text("file_name", "test_document.txt");

    let response = client
        .post(format!("{}/convert2/pdf", TEST_SERVER_URL))
        .multipart(form)
        .send()
        .await
        .expect("Failed to make conversion request");

    assert_eq!(response.status(), 400);

    let error_response: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse error response");

    // Plain-text payloads fail content-type detection before the extension check.
    assert!(
        error_response["error"]
            .as_str()
            .unwrap()
            .contains("Unknown or unsupported file type")
    );
}

#[tokio::test]
async fn test_content_type_mismatch_rejection() {
    ensure_server_running();
    wait_for_server_ready().await;
    let client = reqwest::Client::new();

    // Valid OOXML bytes (xlsx) under a .docx filename — detect_content_type passes,
    // then is_compatible_type rejects on the extension/content mismatch.
    let file_path = get_test_file_path("test_spreadsheet.xlsx");
    let file_content = fs::read(&file_path).expect("Failed to read spreadsheet");

    let form = multipart::Form::new()
        .part(
            "file_content",
            multipart::Part::bytes(file_content)
                .mime_str("application/octet-stream")
                .unwrap()
                .file_name("fake_document.docx"),
        )
        .text("file_name", "fake_document.docx");

    let response = client
        .post(format!("{}/convert2/pdf", TEST_SERVER_URL))
        .multipart(form)
        .send()
        .await
        .expect("Failed to make conversion request");

    assert_eq!(response.status(), 400);

    let error_response: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse error response");

    assert!(
        error_response["error"]
            .as_str()
            .unwrap()
            .contains("File content does not match extension")
    );
}

#[tokio::test]
async fn test_large_file_rejection() {
    ensure_server_running();
    wait_for_server_ready().await;
    let client = reqwest::Client::new();

    // Generated in-memory rather than checked in as a fixture; size exceeds the 100MB validator limit.
    let file_content = vec![0u8; 101 * 1024 * 1024];

    let form = multipart::Form::new()
        .part(
            "file_content",
            multipart::Part::bytes(file_content)
                .mime_str("application/octet-stream")
                .unwrap()
                .file_name("large_file.docx"),
        )
        .text("file_name", "large_file.docx");

    let response = client
        .post(format!("{}/convert2/pdf", TEST_SERVER_URL))
        .multipart(form)
        .send()
        .await
        .expect("Failed to make conversion request");

    assert_eq!(response.status(), 400);

    let error_response: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse error response");

    assert!(
        error_response["error"]
            .as_str()
            .unwrap()
            .contains("File too large")
    );
}

#[tokio::test]
async fn test_invalid_endpoint() {
    ensure_server_running();
    wait_for_server_ready().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/invalid-endpoint", TEST_SERVER_URL))
        .send()
        .await
        .expect("Failed to make request");

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_concurrent_conversions() {
    ensure_server_running();
    wait_for_server_ready().await;
    let client = reqwest::Client::new();

    let file_path = get_test_file_path("test_document.docx");
    let file_content = fs::read(&file_path).expect("Failed to read test document");

    // Test multiple concurrent conversions
    let mut tasks = Vec::new();
    for i in 0..5 {
        let client = client.clone();
        let file_content = file_content.clone();

        let task = tokio::spawn(async move {
            let form = multipart::Form::new()
                .part(
                    "file_content",
                    multipart::Part::bytes(file_content)
                        .mime_str("application/octet-stream")
                        .unwrap()
                        .file_name("test_document.docx"),
                )
                .text("file_name", format!("test_document_{}.docx", i));

            client
                .post(format!("{}/convert2/pdf", TEST_SERVER_URL))
                .multipart(form)
                .send()
                .await
        });

        tasks.push(task);
    }

    let mut success_count = 0;
    for task in tasks {
        if let Ok(response) = task.await.expect("Task failed")
            && response.status().is_success()
        {
            success_count += 1;
            let pdf_content = response.bytes().await.expect("Failed to read PDF response");
            assert!(!pdf_content.is_empty());
            assert_eq!(&pdf_content[0..4], b"%PDF");
        }
    }

    assert!(
        success_count > 0,
        "Expected at least some concurrent conversions to succeed"
    );
}

#[tokio::test]
async fn test_filename_sanitization() {
    ensure_server_running();
    wait_for_server_ready().await;
    let client = reqwest::Client::new();

    let file_path = get_test_file_path("test_document.docx");
    let file_content = fs::read(&file_path).expect("Failed to read test document");

    // Test with dangerous filename
    let form = multipart::Form::new()
        .part(
            "file_content",
            multipart::Part::bytes(file_content)
                .mime_str("application/octet-stream")
                .unwrap()
                .file_name("../../../etc/passwd.docx"),
        )
        .text("file_name", "../../../etc/passwd.docx");

    let response = client
        .post(format!("{}/convert2/pdf", TEST_SERVER_URL))
        .multipart(form)
        .send()
        .await
        .expect("Failed to make conversion request");

    assert_eq!(response.status(), 200);

    // sanitize_filename strips path separators but preserves basename text, so verify only that traversal was neutralized.
    let content_disposition = response.headers().get("content-disposition");
    if let Some(header_value) = content_disposition {
        let header_str = header_value.to_str().unwrap();
        assert!(!header_str.contains("../"));
        assert!(!header_str.contains('/'));
    }
}

#[tokio::test]
async fn test_multiple_file_formats() {
    ensure_server_running();
    wait_for_server_ready().await;
    let client = reqwest::Client::new();

    let test_files = vec!["test_document.docx", "test_spreadsheet.xlsx"];

    for filename in test_files {
        let file_path = get_test_file_path(filename);
        let file_content =
            fs::read(&file_path).unwrap_or_else(|_| panic!("Failed to read {}", filename));

        let form = multipart::Form::new()
            .part(
                "file_content",
                multipart::Part::bytes(file_content)
                    .mime_str("application/octet-stream")
                    .unwrap()
                    .file_name(filename),
            )
            .text("file_name", filename);

        let response = client
            .post(format!("{}/convert2/pdf", TEST_SERVER_URL))
            .multipart(form)
            .send()
            .await
            .expect("Failed to make conversion request");

        assert_eq!(response.status(), 200, "Failed to convert {}", filename);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/pdf"
        );

        let pdf_content = response.bytes().await.expect("Failed to read PDF response");
        assert!(!pdf_content.is_empty(), "Empty PDF for {}", filename);
        assert_eq!(
            &pdf_content[0..4],
            b"%PDF",
            "Invalid PDF header for {}",
            filename
        );
    }
}

#[tokio::test]
async fn test_conversion_timeout_handling() {
    ensure_server_running();
    wait_for_server_ready().await;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5)) // Short timeout to test timeout handling
        .build()
        .expect("Failed to create client");

    let file_path = get_test_file_path("test_document.docx");
    let file_content = fs::read(&file_path).expect("Failed to read test document");

    let form = multipart::Form::new()
        .part(
            "file_content",
            multipart::Part::bytes(file_content)
                .mime_str("application/octet-stream")
                .unwrap()
                .file_name("test_document.docx"),
        )
        .text("file_name", "test_document.docx");

    let response = client
        .post(format!("{}/convert2/pdf", TEST_SERVER_URL))
        .multipart(form)
        .send()
        .await;

    // Either succeeds within timeout or times out - both are valid outcomes
    match response {
        Ok(resp) => {
            assert!(resp.status().is_success() || resp.status().is_server_error());
        }
        Err(e) => {
            // Timeout is acceptable
            assert!(e.is_timeout());
        }
    }
}

#[tokio::test]
async fn test_metrics_accuracy() {
    ensure_server_running();
    wait_for_server_ready().await;
    let client = reqwest::Client::new();

    // Get initial metrics
    let initial_response = client
        .get(format!("{}/metrics", TEST_SERVER_URL))
        .send()
        .await
        .expect("Failed to get initial metrics");

    let initial_metrics: serde_json::Value = initial_response
        .json()
        .await
        .expect("Failed to parse initial metrics");

    let initial_total = initial_metrics["total_conversions"].as_u64().unwrap();
    let initial_successful = initial_metrics["successful_conversions"].as_u64().unwrap();

    // Perform a successful conversion
    let file_path = get_test_file_path("test_document.docx");
    let file_content = fs::read(&file_path).expect("Failed to read test document");

    let form = multipart::Form::new()
        .part(
            "file_content",
            multipart::Part::bytes(file_content)
                .mime_str("application/octet-stream")
                .unwrap()
                .file_name("test_document.docx"),
        )
        .text("file_name", "test_document.docx");

    let conversion_response = client
        .post(format!("{}/convert2/pdf", TEST_SERVER_URL))
        .multipart(form)
        .send()
        .await
        .expect("Failed to make conversion request");

    assert_eq!(conversion_response.status(), 200);

    // Get updated metrics
    let updated_response = client
        .get(format!("{}/metrics", TEST_SERVER_URL))
        .send()
        .await
        .expect("Failed to get updated metrics");

    let updated_metrics: serde_json::Value = updated_response
        .json()
        .await
        .expect("Failed to parse updated metrics");

    let updated_total = updated_metrics["total_conversions"].as_u64().unwrap();
    let updated_successful = updated_metrics["successful_conversions"].as_u64().unwrap();

    // Verify metrics increased
    assert!(
        updated_total > initial_total,
        "Total conversions should have increased"
    );
    assert!(
        updated_successful > initial_successful,
        "Successful conversions should have increased"
    );
}

#[tokio::test]
async fn test_health_status_changes() {
    ensure_server_running();
    wait_for_server_ready().await;
    let client = reqwest::Client::new();

    let health_response = client
        .get(format!("{}/health", TEST_SERVER_URL))
        .send()
        .await
        .expect("Failed to get health status");

    assert_eq!(health_response.status(), 200);

    let health_data: serde_json::Value = health_response
        .json()
        .await
        .expect("Failed to parse health response");

    // Health status should be one of the expected values
    let status = health_data["status"].as_str().unwrap();
    assert!(
        status == "healthy" || status == "degraded" || status == "unhealthy",
        "Unexpected health status: {}",
        status
    );

    // Verify health data structure
    assert!(health_data["timestamp"].is_string());
    assert!(health_data["version"].is_string());
    assert!(health_data["uptime_seconds"].is_number());
    assert!(health_data["metrics"]["total_conversions"].is_number());
    assert!(health_data["metrics"]["successful_conversions"].is_number());
    assert!(health_data["metrics"]["failed_conversions"].is_number());
}
