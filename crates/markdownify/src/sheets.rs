use std::{
    fs::File,
    io::{self, BufRead},
    path::Path,
};

use calamine::Reader;

fn detect_delimiter(line: &str) -> u8 {
    let candidates = [',', ';', '\t', '|'];
    candidates
        .iter()
        .map(|&c| (c, line.matches(c).count()))
        .max_by_key(|&(_, count)| count)
        .map(|(c, _)| c as u8)
        .unwrap_or(b',') // fallback to comma
}

/// creates a markdown table
/// # usage:
/// ```
/// use markdownify::sheets::to_markdown_table;
///
/// let headers = vec!["Names".to_string(), "Salary".to_string()];
/// let rows = vec![
///     vec!["Sarah".to_string(), "100".to_string()],
///     vec!["Jeff".to_string(), "200".to_string()],
/// ];
/// let md = to_markdown_table(&headers, &rows);
/// println!("{}", md);
/// ```
pub fn to_markdown_table(headers: &[String], rows: &[Vec<String>]) -> String {
    let mut output = String::new();
    output += &format!("| {} |\n", headers.join(" | "));
    output += &format!("|{}|\n", vec!["---"; headers.len()].join("|"));

    for row in rows {
        output += &format!("| {} |\n", row.join(" | "));
    }

    output
}

/// convert `xlsx` | `xls` | `xlsm` | `xlsb` | `xla` | `xlam` | `ods` files into markdown
/// # usage:
/// ```
/// use std::path::Path;
/// use markdownify::sheets::sheets_convert;
///
/// let path = Path::new("path/to/file.xlsx");
/// match sheets_convert(&path) {
///     Ok(md) => println!("{}", md),
///     Err(e) => eprintln!("Error: {}", e)
/// }
/// ```
pub fn sheets_convert(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut workbook = calamine::open_workbook_auto(path)?;
    let mut output = String::new();

    for sheet_name in workbook.sheet_names() {
        if let Ok(range) = workbook.worksheet_range(&sheet_name) {
            let mut rows = range.rows();
            if let Some(header_row) = rows.next() {
                let headers = header_row
                    .iter()
                    .map(|cell| cell.to_string())
                    .collect::<Vec<_>>();
                let body = rows
                    .map(|r| r.iter().map(|cell| cell.to_string()).collect::<Vec<_>>())
                    .collect::<Vec<_>>();

                output += &format!("# {}\n\n", sheet_name);
                output += &to_markdown_table(&headers, &body);
                output += "\n";
            }
        }
    }

    if output.is_empty() {
        Err("No readable sheets found.".into())
    } else {
        Ok(output)
    }
}

/// convert `csv` into markdown
/// # usage:
/// ```
/// use std::path::Path;
/// use markdownify::sheets::csv_converter;
///
/// let path = Path::new("path/to/file.csv");
/// match csv_converter(&path) {
///     Ok(md) => println!("{}", md),
///     Err(e) => eprintln!("Error: {}", e)
/// }
/// ```
pub fn csv_converter(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let mut first_line = String::new();
    let _ = io::BufReader::new(&mut file).read_line(&mut first_line)?;

    let delimiter = detect_delimiter(&first_line);
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .from_path(path)?;

    let headers = reader
        .headers()?
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    let rows = reader
        .records()
        .map(|r| r.map(|rec| rec.iter().map(|s| s.to_string()).collect::<Vec<_>>()))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(to_markdown_table(&headers, &rows))
}
