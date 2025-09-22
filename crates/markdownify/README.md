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
    // Convert a file to markdown using just a path
    let path = Path::new("document.docx");
    let markdown = convert(path)?;
    println!("{}", markdown);

    // With a name header
    let opts = ConvertOptions::new("document.docx")
        .with_name_header("My Document");
    let markdown = convert(opts)?;

    // For PDFs with custom screen size
    let opts = ConvertOptions::new("document.pdf")
        .with_name_header("My PDF")
        .with_screen_size((100, 20)); // width, height in cells
    let markdown = convert(opts)?;
    
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
