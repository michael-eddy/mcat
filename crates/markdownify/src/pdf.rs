use std::path::Path;

/// convert `pdf` into markdown
/// # usage:
/// ```
/// use std::path::Path;
/// use markdownify::pdf::pdf_convert;
///
/// let path = Path::new("path/to/file.pdf");
/// match pdf_convert(&path) {
///     Ok(md) => println!("{}", md),
///     Err(e) => eprintln!("Error: {}", e)
/// }
/// ```
pub fn pdf_convert(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    todo!();
}
