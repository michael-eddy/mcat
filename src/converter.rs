use std::{collections::HashSet, env, path::Path};

pub use pyo3::types::PyModule;
use pyo3::{prelude::*, prepare_freethreaded_python};

pub fn markitdown_convert(input: &str) -> PyResult<String> {
    unsafe {
        env::set_var("PYTHONWARNINGS", "ignore");
    }
    prepare_freethreaded_python();
    Python::with_gil(|py| {
        // Attempt to import 'markitdown'
        let result = PyModule::import(py, "markitdown");

        if result.is_err() {
            // If import fails, install 'markitdown' using pip
            let subprocess = PyModule::import(py, "subprocess")?;
            subprocess.call_method1(
                "check_call",
                (vec![
                    "python".to_string(),
                    "-m".to_string(),
                    "pip".to_string(),
                    "install".to_string(),
                    "markitdown[all]".to_string(),
                    "--quiet".to_string(),
                ],),
            )?;
        }

        // silent
        let io = PyModule::import(py, "io")?;
        let sys = PyModule::import(py, "sys")?;
        let devnull = io.getattr("StringIO")?.call0()?; // or '/dev/null' for suppression
        sys.setattr("stdout", &devnull)?;
        sys.setattr("stderr", &devnull)?;

        let markitdown = PyModule::import(py, "markitdown")?;
        let converter = markitdown.getattr("MarkItDown")?.call0()?;
        let result = converter.call_method1("convert", (input,))?;
        let text_content: String = result.getattr("text_content")?.extract()?;

        Ok(text_content)
    })
}

pub fn is_markitdown_supported(path: &Path) -> bool {
    let extension = match path.extension() {
        Some(ext) => ext.to_string_lossy().to_lowercase(),
        None => return false,
    };

    // Create a HashSet of supported formats/extensions for markitdown
    let supported_formats: HashSet<&str> = [
        "docx", "doc", "dotx", "dot", // Word documents
        "pdf", "zip", "epub", //others
        "xlsx", "xls", "xlsm", // Excel spreadsheets
        "pptx", "ppt", "pptm", // PowerPoint presentations
        "odt", "ods", "odp", // OpenDocument formats
    ]
    .iter()
    .cloned()
    .collect();

    supported_formats.contains(extension.as_str())
}
