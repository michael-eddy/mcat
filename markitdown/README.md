# markitdown

A Rust library for converting various document formats to Markdown, part of the [mcat](https://github.com/Skardyy/mcat) project.

[![Crates.io](https://img.shields.io/crates/v/mcat-markitdown.svg)](https://crates.io/crates/mcat-markitdown)
[![Documentation](https://docs.rs/mcat-markitdown/badge.svg)](https://docs.rs/mcat-markitdown)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Overview

markitdown is a Rust implementation inspired by Microsoft's [markitdown](https://github.com/microsoft/markitdown) Python project. It provides functionality to convert various document formats to Markdown, making them easier to view, share, and integrate into AI prompts.

> **Note:** As part of the mcat project, this crate's API may change. Use with caution in production environments.

## Supported Formats

| Format | Extension | Description |
|--------|-----------|-------------|
| Word Documents | .docx | Microsoft Word documents |
| OpenDocument Text | .odt | OpenDocument text files |
| PDF | .pdf | Portable Document Format files |
| PowerPoint | .pptx | Microsoft PowerPoint presentations |
| Excel/Spreadsheets | .xlsx, .xls, .xlsm, .xlsb, .xla, .xlam, .ods | Various spreadsheet formats |
| CSV | .csv | Comma-separated values (auto-detects delimiter) |
| ZIP Archives | .zip | Extracts and converts contained files |
| Markdown | .md | Passes through with formatting |
| HTML | .html | Passes through with formatting |
| Other text formats | (various) | Falls back to code block formatting |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
mcat-markitdown = "0.1.0"
```

## Usage

### Basic Usage

```rust
use std::path::Path;
use mcat_markitdown::convert;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Convert a file to markdown
    let markdown = convert(Path::new("document.docx"), None)?;
    println!("{}", markdown);
    
    // With an optional name header
    let name = String::from("My Document");
    let markdown = convert(Path::new("spreadsheet.xlsx"), Some(&name))?;
    println!("{}", markdown);
    
    Ok(())
}
```

### Working with Specific Formats

You can also use the format-specific converters directly:

```rust
use std::path::Path;
use mcat_markitdown::{docx, pdf, sheets, pptx, opendoc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Convert a Word document
    let markdown = docx::docx_convert(Path::new("document.docx"))?;
    
    // Convert a PDF
    let markdown = pdf::pdf_convert(Path::new("document.pdf"))?;
    
    // Convert a CSV or Excel file
    let markdown = sheets::csv_converter(Path::new("data.csv"))?;
    let markdown = sheets::sheets_convert(Path::new("data.xlsx"))?;
    
    // Convert a PowerPoint presentation
    let markdown = pptx::pptx_converter(Path::new("presentation.pptx"))?;
    
    // Convert an OpenDocument file
    let markdown = opendoc::opendoc_convert(Path::new("document.odt"))?;
    
    Ok(())
}
```

### Working with ZIP Files

The library can recursively convert files within ZIP archives:

```rust
use std::path::Path;
use mcat_markitdown::zip_convert;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let markdown = zip_convert(Path::new("archive.zip"))?;
    println!("{}", markdown);
    
    Ok(())
}
```

## Features

- **Smart Structure Preservation**: Maintains document structure including headings, lists, and tables
- **Text Styling**: Preserves bold, italic, underline, and other text formatting
- **Table Support**: Converts tables from various formats to Markdown tables
- **Auto Format Detection**: Automatically handles different file formats
- **ZIP Extraction**: Processes multiple files from ZIP archives
- **Fallback Mode**: Gracefully handles unknown formats

## License

This project is licensed under the MIT License - see the LICENSE file for details.
