# DocOxide Test Suite

Integration tests for the DocOxide document conversion service.

## Test Structure

### Integration Tests (`integration_tests.rs`)

The integration tests boot the HTTP server in-process (via a thread, sharing a single `OnceLock`-guarded instance across all tests) and exercise it through `reqwest`.

#### Health & Monitoring
- **Health Endpoint**: `/health` returns proper health status
- **Metrics Endpoint**: `/metrics` returns conversion statistics
- **Health Status Changes**: status calculation based on success rates
- **Metrics Accuracy**: metrics increment correctly across requests

#### Document Conversion
- **DOCX Conversion**: Word → PDF
- **XLSX Conversion**: Excel → PDF
- **PDF/A Conversion**: PDF/A output format
- **Multiple File Formats**: covers the supported Office formats

#### Security & Validation
- **Empty File Rejection**: empty payloads are rejected
- **Invalid File Type Rejection**: plain-text and other unsupported types are rejected
- **Content Type Mismatch**: extension/content mismatch is rejected
- **Large File Rejection**: payloads above the validator's 100MB limit are rejected (the test allocates the oversized buffer in-memory; no fixture is checked in)
- **Filename Sanitization**: path-traversal characters are stripped

#### Reliability
- **Concurrent Conversions**: multiple simultaneous requests
- **Conversion Timeout Handling**: client-side timeout behavior
- **Invalid Endpoints**: 404 for unknown URLs

## Test Data

`fixtures/` contains:

- **test_document.docx** — valid Word document
- **test_spreadsheet.xlsx** — valid Excel spreadsheet
- **test_document.txt** — plain-text payload (for unsupported-type rejection)
- **fake_document.docx** — `.docx` extension with non-Office content
- **empty_file.docx** — zero-byte file

The oversized payload used by `test_large_file_rejection` is generated at runtime; it is intentionally not checked in.

## Running Tests

### Prerequisites

Tests require LibreOffice (both the LibreOfficeKit headers for compilation and the LibreOffice runtime at `/usr/lib/libreoffice/program`). Easiest path is Docker.

### Docker (recommended)

```bash
docker build -f Dockerfile.test -t docoxide-test .
docker run --rm docoxide-test
```

### Local

If you have LibreOfficeKit dev headers and LibreOffice installed at `/usr/lib/libreoffice/program`:

```bash
LO_INCLUDE_PATH=/path/to/LibreOfficeKit cargo test --test integration_tests
```

Specific tests:
```bash
cargo test --test integration_tests test_health
cargo test --test integration_tests test_successful
```

## Test Configuration

- **Server port**: 8765 (constant in `integration_tests.rs`)
- **Server boot wait**: up to 30 × 500ms while polling `/health`
- **LibreOffice path**: `/usr/lib/libreoffice/program` (matches the Docker image)

## Expected Behavior

### Successful Conversions
- 200 status, `Content-Type: application/pdf`
- Body starts with `%PDF`
- `Content-Disposition` carries the sanitized filename

### Validation Failures
- 400 status with a JSON body of the form `{"error": "...", "status": 400}`
- Specific messages: `"Empty file"`, `"File too large: ..."`, `"Unknown or unsupported file type"`, `"File content does not match extension"`
