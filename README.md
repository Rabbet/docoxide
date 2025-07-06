# docoxide

A lightweight HTTP service for converting documents to PDF using LibreOffice.

## Features

- Convert documents to PDF via HTTP API
- Support for common document formats (docx, xlsx, pptx, etc.)
- Built-in validation and metrics
- Configurable LibreOffice installation path

## Usage

```bash
# Run with default settings
cargo run

# Specify custom LibreOffice path and port
cargo run -- --office-path /path/to/libreoffice --port 8080
```

## Requirements

- LibreOffice installed
- Rust 1.70+
