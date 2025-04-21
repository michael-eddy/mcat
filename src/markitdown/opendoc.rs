use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::io::{Cursor, Read};
use std::path::Path;
use zip::ZipArchive;

use super::sheets;

#[derive(Debug)]
enum OpenEvent {
    P,
    H,
    A,
    Span,
    Table,
    TableRow,
    TableCell,
    TableText,
    List,
    ListItem,
    ListText,
    None,
}

pub fn opendoc_convert(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let data = std::fs::read(path)?;
    let cursor = Cursor::new(data);
    let mut archive = ZipArchive::new(cursor)?;
    let mut xml_content = String::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if file.name() == "content.xml" {
            file.read_to_string(&mut xml_content)?;
            break;
        }
    }

    let mut reader = Reader::from_str(&xml_content);
    let mut buf = Vec::new();
    let mut markdown = String::new();
    let mut event = OpenEvent::None;
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut start = true;

    loop {
        let mut set = |e| {
            match e {
                OpenEvent::None => start = false,
                _ => start = true,
            }
            if start {
                match event {
                    OpenEvent::TableCell | OpenEvent::TableText => event = OpenEvent::TableText,
                    OpenEvent::List => event = OpenEvent::ListText,
                    OpenEvent::ListItem => event = OpenEvent::ListText,
                    OpenEvent::Table => {
                        event = e;
                    }
                    _ => event = e,
                }
            } else {
                event = e;
            }
        };
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                // maybe parsing attributes for more
                // info, but good enough imo
                b"text:p" => set(OpenEvent::P),
                b"text:h" => set(OpenEvent::H),
                b"text:span" => set(OpenEvent::Span),
                b"table:table" => set(OpenEvent::Table),
                b"table:table-row" => set(OpenEvent::TableRow),
                b"table:table-cell" => set(OpenEvent::TableCell),
                b"text:list" => set(OpenEvent::List),
                b"text:list-item" => set(OpenEvent::ListItem),
                b"text:a" => set(OpenEvent::A),
                _ => {
                    // eprintln!("start {}", String::from_utf8(e.name().0.to_vec())?)
                }
            },
            Ok(Event::Text(e)) => {
                let text = &e.unescape()?.into_owned();
                let text = match event {
                    OpenEvent::P => &format!("{}\n\n", text),
                    OpenEvent::H => &format!("### {}\n\n", text),
                    OpenEvent::A => continue, //probs at the attributes, idc that much
                    OpenEvent::Span => &format!(" {} ", text),
                    OpenEvent::TableText => {
                        current_row.push(text.into());
                        continue;
                    }
                    OpenEvent::ListText => &format!(" * {}  \n", text),
                    _ => continue,
                };
                markdown.push_str(text);
            }
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"table:table" => {
                    let headers = table_rows[0].clone();
                    let data_rows = if table_rows.len() > 1 {
                        table_rows[1..].to_vec()
                    } else {
                        Vec::new()
                    };
                    set(OpenEvent::None);
                    markdown.push_str(&sheets::to_markdown_table(&headers, &data_rows));
                    markdown.push_str("\n");
                    table_rows = Vec::new();
                }
                b"table:table-row" => {
                    table_rows.push(current_row);
                    current_row = Vec::new();
                    set(OpenEvent::None);
                }
                b"text:p" | b"text:h" | b"text:list" | b"text:list-item" => {
                    if markdown.chars().last().unwrap_or_default() != '\n' {
                        markdown.push_str("\n\n");
                    }
                    set(OpenEvent::None);
                }
                _ => {
                    set(OpenEvent::None);
                }
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

    Ok(markdown.trim().to_string())
}
