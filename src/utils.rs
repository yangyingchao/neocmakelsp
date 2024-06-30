mod findpackage;
pub mod treehelper;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Serialize, Clone)]
pub enum FileType {
    Dir,
    File,
}
impl std::fmt::Display for FileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileType::Dir => write!(f, "Dir"),
            FileType::File => write!(f, "File"),
        }
    }
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct CMakePackage {
    pub name: String,
    pub filetype: FileType,
    pub filepath: String,
    pub version: Option<String>,
    pub tojump: Vec<PathBuf>,
}

pub use findpackage::*;
use tree_sitter::Node;

pub fn get_range_content(
    source: &[&str],
    row_start: usize,
    column_start: usize,
    row_end: usize,
    column_end: usize,
) -> String {
    let mut content: String;

    if row_start == row_end {
        assert!(column_start <= column_end);
        content = source[row_start][column_start..column_end].to_string();
    } else {
        let mut row = row_start;
        content = source[row][column_start..].to_string();
        row += 1;

        while row < row_end {
            content = format!("{}\n{}", content, source[row]);
            row += 1;
        }

        if row != row_start {
            assert_eq!(row, row_end);
            content = format!("{}\n{}", content, &source[row][..column_end])
        }
    }
    content
}

pub fn get_node_content(source: &[&str], node: &Node) -> String {
    get_range_content(
        source,
        node.start_position().row,
        node.start_position().column,
        node.end_position().row,
        node.end_position().column,
    )
}

pub fn execute_command(
    command: &str,
    args: &[&str],
) -> Result<(i32, String, String), std::io::Error> {
    let child_process = Command::new(command)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let output = child_process.wait_with_output()?;
    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok((exit_code, stdout, stderr))
}

#[test]
fn test_command_execution() {
    // normal command, should not error.
    if let Ok(result) = execute_command("ls", &["-a", "-l"]) {
        let (code, out, err) = result;
        assert_eq!(code, 0);
        assert!(!out.is_empty());
        assert!(err.is_empty());
    } else {
        assert!(false);
    };

    if let Ok(result) = execute_command("ls", &["-a", "-l", "/target_dir_does_not_exist"]) {
        let (code, out, err) = result;
        assert_ne!(code, 0);
        assert!(out.is_empty());
        assert!(!err.is_empty());
    } else {
        assert!(false);
    };

    if let Err(err) = execute_command("command_not_exist", &["-a", "-l"]) {
        println!("{}", err);
    } else {
        assert!(false);
    };
}
