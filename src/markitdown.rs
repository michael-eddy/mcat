use std::error;
use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;

pub struct MarkitdownProcess {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl MarkitdownProcess {
    fn new() -> std::io::Result<Self> {
        let python_command = if cfg!(target_os = "windows") {
            "python"
        } else {
            "python3"
        };
        let python_script = include_str!("../scripts/markitdown_worker.py");

        let mut child = Command::new(python_command)
            .arg("-c")
            .arg(python_script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let stdin = child.stdin.take().expect("Failed to open stdin");
        let stdout = BufReader::new(child.stdout.take().expect("Failed to open stdout"));

        Ok(MarkitdownProcess { stdin, stdout })
    }

    pub fn convert(&mut self, path: &str) -> Result<String, Box<dyn error::Error>> {
        writeln!(self.stdin, "{}", path)?;
        self.stdin.flush()?;

        let mut output = Vec::new();
        self.stdout.read_until(b'\0', &mut output)?;

        if !output.is_empty() && output[output.len() - 1] == b'\0' {
            output.pop();
        }

        let res = String::from_utf8(output)?;
        Ok(res)
    }
}

lazy_static! {
    pub static ref MARKITDOWN: Mutex<MarkitdownProcess> = {
        let process = MarkitdownProcess::new().expect("Failed to start markitdown process");
        Mutex::new(process)
    };
}
