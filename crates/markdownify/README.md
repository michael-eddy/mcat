# markdownify

A Rust library for converting various document formats to Markdown, part of the [mcat](https://github.com/Skardyy/mcat) project.

[![Crates.io](https://img.shields.io/crates/v/markdownify.svg)](https://crates.io/crates/markdownify)
[![Documentation](https://docs.rs/markdownify/badge.svg)](https://docs.rs/markdownify)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Supported Formats

| Format | Extension |
|--------|-----------|
| Word Documents | .docx |
| OpenDocument Text | .odt, .odp |
| PDF | .pdf |
| PowerPoint | .pptx |
| Excel/Spreadsheets | .xlsx, .xls, .xlsm, .xlsb, .xla, .xlam, .ods |
| CSV | .csv |
| ZIP Archives | .zip |
| Other text formats | (various) Falls back to code block formatting |

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
