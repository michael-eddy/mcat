/// convert `pdf` into markdown
/// # usuage:
/// ```
/// let path = Path::new("path/to/file.pdf");
/// let md = pdf_convert(&path).unwrap();
/// println!("{}", md);
/// ```
pub fn pdf_convert(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    todo!();
}
