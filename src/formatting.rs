use std::{path::Path, process::Command};

use async_lsp::lsp_types::{self, Range};
use lsp_types::{Position, TextEdit};
use tempfile::NamedTempFile;
use std::io::Write;

use crate::utils::get_range_content;

pub fn getformat(path: &Path) -> Option<Vec<TextEdit>> {
    if !path.exists() {
        return None;
    }

    let output = Command::new("cmake-format").arg(path).output().ok()?;
    let new_text = String::from_utf8_lossy(&output.stdout);

    Some(vec![TextEdit {
        range: lsp_types::Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: new_text.len() as u32,
                character: 0,
            },
        },
        new_text: new_text.to_string(),
    }])
}

pub fn format_range(content: &String, range: Range) -> Option<Vec<TextEdit>> {
    let source: Vec<&str> = content.as_str().split("\n").collect();
    let source = get_range_content(&source, range.start.line as usize, range.start.character as usize,
        range.end.line as usize , range.end.character as usize);

    // Create a file inside of `std::env::temp_dir()`.
    let named_temp_file = NamedTempFile::new().unwrap();
    named_temp_file.as_file().write(source.as_bytes()).unwrap();
    named_temp_file.as_file().flush().unwrap();

    let output = Command::new("cmake-format").arg(named_temp_file.path()).output().ok()?;
    let new_text = String::from_utf8_lossy(&output.stdout);

    Some(vec![TextEdit {
        range: lsp_types::Range {
            start: range.start,
            end: range.end,
        },
        new_text: new_text.to_string(),
    }])
}
