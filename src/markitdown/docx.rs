use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::io::{Cursor, Read};
use std::path::Path;
use zip::ZipArchive;

use super::sheets;

pub fn docx_convert(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    // Read the document file into memory
    let data = std::fs::read(path)?;
    let cursor = Cursor::new(data);

    // Open the .docx as a zip archive
    let mut archive = ZipArchive::new(cursor)?;
    let mut xml_content = String::new();

    // Extract the document.xml which contains the main content
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if file.name() == "word/document.xml" {
            file.read_to_string(&mut xml_content)?;
            break;
        }
    }

    let mut reader = Reader::from_str(&xml_content);
    let mut buf = Vec::new();
    let mut markdown = String::new();

    // Tracking state
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut is_table = false;
    let mut is_list_item = 0; // 0: not in list, 1: in list needs *, 2: done writing list item

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"w:tbl" => is_table = true,
                b"w:tr" => continue,
                b"w:tc" => continue,
                b"w:numPr" => is_list_item = 1, // Number properties indicate a list item
                _ => {
                    eprintln!("{}", String::from_utf8(e.name().as_ref().to_vec())?);
                }
            },
            Ok(Event::Text(e)) => {
                let text = &e.unescape()?.into_owned();
                if is_table {
                    current_row.push(text.into());
                } else if is_list_item == 1 {
                    markdown.push_str(&format!(" * {}", text));
                    is_list_item = 2;
                } else {
                    markdown.push_str(text);
                }
            }
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"w:tbl" => {
                    if !table_rows.is_empty() {
                        let headers = table_rows[0].clone();
                        let data_rows = if table_rows.len() > 1 {
                            table_rows[1..].to_vec()
                        } else {
                            Vec::new()
                        };
                        is_table = false;
                        markdown.push_str(&sheets::to_markdown_table(&headers, &data_rows));
                        markdown.push_str("\n");
                        table_rows = Vec::new();
                    }
                }
                b"w:tr" => {
                    table_rows.push(current_row);
                    current_row = Vec::new();
                }
                b"w:numPr" => continue,
                b"w:listItem" => {
                    is_list_item = 0;
                    markdown.push_str("  \n");
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(
                    format!("Error at position {}: {:?}", reader.buffer_position(), e).into(),
                );
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(format(&markdown))
}

// Same format function from your ODT implementation
fn format(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut newline_count = 0;
    let mut spaces_count = 0;

    for line in input.lines() {
        if line.trim() == "" {
            result.push_str("\n");
        } else {
            result.push_str(&format!("{}\n", line));
        }
    }
    let input = &result;
    let mut result = String::with_capacity(input.len());

    for c in input.chars() {
        if c == ' ' {
            spaces_count += 1;
        }
        if c == '\n' {
            newline_count += 1;
            if spaces_count >= 2 {
                newline_count += 1;
            }
            spaces_count = 0;
            if newline_count <= 2 {
                result.push(c);
            }
        } else {
            newline_count = 0;
            spaces_count = 0;
            result.push(c);
        }
    }

    result
}
