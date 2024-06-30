use std::path::Path;

use async_lsp::lsp_types::{self, Range};
use lsp_types::{Position, TextEdit};
use std::io::Write;
use tempfile::NamedTempFile;

use crate::utils::{execute_command, get_range_content};

pub fn format_range(content: &str, range: Range) -> Result<Option<Vec<TextEdit>>, String> {
    let source: Vec<&str> = content.split('\n').collect();
    let source = get_range_content(
        &source,
        range.start.line as usize,
        range.start.character as usize,
        range.end.line as usize,
        range.end.character as usize,
    );

    // Create a file inside of `std::env::temp_dir()`.
    let named_temp_file = NamedTempFile::new().unwrap();
    let _ = named_temp_file.as_file().write(source.as_bytes()).unwrap();
    named_temp_file.as_file().flush().unwrap();

    match execute_command("cmake-format", &[named_temp_file.path().to_str().unwrap()]) {
        Err(err) => Err(err.to_string()),
        Ok(result) => {
            let (code, out, err) = result;
            if code == 0 {
                Ok(Some(vec![TextEdit {
                    range: lsp_types::Range {
                        start: range.start,
                        end: range.end,
                    },
                    new_text: out,
                }]))
            } else {
                Err(err)
            }
        }
    }
}

pub fn getformat(path: &Path) -> Result<Option<Vec<TextEdit>>, String> {
    if !path.exists() {
        return Err(format!("File {:?} does not exist", path));
    }

    match execute_command("cmake-format", &[path.to_str().unwrap()]) {
        Err(err) => Err(err.to_string()),
        Ok(result) => {
            let (code, out, err) = result;
            if code == 0 {
                Ok(Some(vec![TextEdit {
                    range: lsp_types::Range {
                        start: Position {
                            line: 0,
                            character: 0,
                        },
                        end: Position {
                            line: out.len() as u32,
                            character: 0,
                        },
                    },
                    new_text: out,
                }]))
            } else {
                Err(err)
            }
        }
    }
}
