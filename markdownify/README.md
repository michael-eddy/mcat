# markdownify

A Rust library for converting various document formats to Markdown, part of the [mcat](https://github.com/Skardyy/mcat) project.

[![Crates.io](https://img.shields.io/crates/v/markdownify.svg)](https://crates.io/crates/markdownify)
[![Documentation](https://docs.rs/markdownify/badge.svg)](https://docs.rs/markdownify)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Overview

markdownify is a Rust implementation inspired by Microsoft's [markitdown](https://github.com/microsoft/markitdown) Python project. It provides functionality to convert various document formats to Markdown, making them easier to view, share, and integrate into AI prompts.

## Supported Formats

| Format | Extension | Description |
|--------|-----------|-------------|
| Word Documents | .docx | Microsoft Word documents |
| OpenDocument Text | .odt, .odp | OpenDocument text files |
| PDF | .pdf | Portable Document Format files |
| PowerPoint | .pptx | Microsoft PowerPoint presentations |
| Excel/Spreadsheets | .xlsx, .xls, .xlsm, .xlsb, .xla, .xlam, .ods | Various spreadsheet formats |
| CSV | .csv | Comma-separated values (auto-detects delimiter) |
| ZIP Archives | .zip | Extracts and converts contained files |
| Other text formats | (various) | Falls back to code block formatting |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
markdownify = "0.1.1"
```

## Usage

### Basic Usage

```rust
use std::path::Path;
use markdownify::convert;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Convert a file to markdown
    let path = Path::new("document.docx");
    let markdown = convert(&path, None)?;
    println!("{}", markdown);
    
    // With an optional name header
    let name = String::from("My Spreadsheet");
    let path = Path::new("spreadsheet.xlsx");
    let markdown = convert(&path, Some(&name))?;
    println!("{}", markdown);
    
    Ok(())
}
```

### Working with Specific Formats

You can also use the format-specific converters directly:

```rust
use std::path::Path;
use markdownify::{docx, pdf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Convert a Word document
    let path = Path::new("document.docx")
    let markdown = docx::docx_convert(&path)?;
    
    // Convert a PDF
    let path = Path::new("document.pdf")
    let markdown = pdf::pdf_convert(&path)?;
    
    // same for the others..
    
    Ok(())
}
```

## License

This project is licensed under the MIT License - see the LICENSE under mcat for details.
