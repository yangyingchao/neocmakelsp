/// buildin Commands and vars
use anyhow::Result;
use once_cell::sync::Lazy;
use std::process::Command;
use std::{collections::HashMap, iter::zip};
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, Documentation, InsertTextFormat};

use crate::consts::TREESITTER_CMAKE_LANGUAGE;
use crate::languageserver::get_client_capabilities;
use crate::utils::get_node_content;

/// convert input text to a snippet, if possible.
fn convert_to_lsp_snippet(key: &str, input: &str) -> String {
    let mut parse = tree_sitter::Parser::new();
    parse.set_language(&TREESITTER_CMAKE_LANGUAGE).unwrap();
    let tree = parse.parse(input, None).unwrap();
    let mut node = tree.root_node().child(0).unwrap();
    if node.kind_id() == 78 {
        // TODO: remove hard-coded 78, which means normal_command
        let mut v: Vec<String> = vec![];
        let mut i = 0;
        node = node.child(2).unwrap();
        if node.kind_id() == 57 {
            // argument_list
            let source: Vec<&str> = input.split('\n').collect();
            node = node.child(0).unwrap();
            loop {
                if node.kind_id() == 48 {
                    i += 1;
                    v.push(format!("${{{}:{}}}", i, get_node_content(&source, &node)));
                }
                match node.next_sibling() {
                    Some(c) => node = c,
                    _ => break,
                };
            }
            return format!("{}({})", key, v.join(" "));
        }
    }
    input.to_string()
}

/// CMake build in commands
pub static BUILDIN_COMMAND: Lazy<Result<Vec<CompletionItem>>> = Lazy::new(|| {
    let re = regex::Regex::new(r"[a-zA-z]+\n-+").unwrap();
    let output = Command::new("cmake")
        .arg("--help-commands")
        .output()?
        .stdout;
    let temp = String::from_utf8_lossy(&output);
    let keys: Vec<_> = re
        .find_iter(&temp)
        .map(|message| {
            let temp: Vec<&str> = message.as_str().split('\n').collect();
            temp[0]
        })
        .collect();
    let contents: Vec<_> = re.split(&temp).collect();
    let contents = &contents[1..].to_vec();

    let mut completes = HashMap::new();
    for (key, content) in keys.iter().zip(contents) {
        let small_key = key.to_lowercase();
        let big_key = key.to_uppercase();
        completes.insert(small_key, content.to_string());
        completes.insert(big_key, content.to_string());
    }
    #[cfg(unix)]
    {
        completes.insert(
            "pkg_check_modules".to_string(),
            "please findpackage PkgConfig first".to_string(),
        );
        completes.insert(
            "PKG_CHECK_MODULES".to_string(),
            "please findpackage PkgConfig first".to_string(),
        );
    }

    let client_support_snippet = match get_client_capabilities() {
        Some(c) => c
            .completion
            .unwrap()
            .completion_item
            .unwrap()
            .snippet_support
            .unwrap_or(false),
        _ => false,
    };

    Ok(completes
        .iter()
        .map(|(akey, message)| {
            let mut kind = CompletionItemKind::FUNCTION;
            let mut insert_text_format = InsertTextFormat::PLAIN_TEXT;
            let mut label = akey.to_string();
            let s = format!(r"\n\s+(?P<signature>{}\([^)]*\))", akey);
            let r_match_signature = regex::Regex::new(s.as_str()).unwrap();

            // snippets only work for lower case for now...
            if client_support_snippet && label.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
                label = match r_match_signature.captures(message) {
                    Some(m) => {
                        insert_text_format = InsertTextFormat::SNIPPET;
                        kind = CompletionItemKind::SNIPPET;
                        convert_to_lsp_snippet(akey, m.name("signature").unwrap().as_str())
                    }
                    _ => akey.to_string(),
                }
            };

            CompletionItem {
                label: label.clone(),
                kind: Some(kind),
                detail: Some("Function".to_string()),
                documentation: Some(Documentation::String(message.to_string())),
                insert_text: Some(label),
                insert_text_format: Some(insert_text_format),
                ..Default::default()
            }
        })
        .collect())
});

/// cmake buildin vars
pub static BUILDIN_VARIABLE: Lazy<Result<Vec<CompletionItem>>> = Lazy::new(|| {
    let re = regex::Regex::new(r"[z-zA-z]+\n-+").unwrap();
    let output = Command::new("cmake")
        .arg("--help-variables")
        .output()?
        .stdout;
    let temp = String::from_utf8_lossy(&output);
    let key: Vec<_> = re
        .find_iter(&temp)
        .map(|message| {
            let temp: Vec<&str> = message.as_str().split('\n').collect();
            temp[0]
        })
        .collect();
    let content: Vec<_> = re.split(&temp).collect();
    let context = &content[1..];

    Ok(zip(key, context)
        .map(|(akey, message)| CompletionItem {
            label: akey.to_string(),
            kind: Some(CompletionItemKind::VARIABLE),
            detail: Some("Variable".to_string()),
            documentation: Some(Documentation::String(message.to_string())),
            ..Default::default()
        })
        .collect())
});

/// Cmake buildin modules
pub static BUILDIN_MODULE: Lazy<Result<Vec<CompletionItem>>> = Lazy::new(|| {
    let re = regex::Regex::new(r"[z-zA-z]+\n-+").unwrap();
    let output = Command::new("cmake").arg("--help-modules").output()?.stdout;
    let temp = String::from_utf8_lossy(&output);
    let key: Vec<_> = re
        .find_iter(&temp)
        .map(|message| {
            let temp: Vec<&str> = message.as_str().split('\n').collect();
            temp[0]
        })
        .collect();
    let content: Vec<_> = re.split(&temp).collect();
    let context = &content[1..];
    Ok(zip(key, context)
        .map(|(akey, message)| CompletionItem {
            label: akey.to_string(),
            kind: Some(CompletionItemKind::MODULE),
            detail: Some("Module".to_string()),
            documentation: Some(Documentation::String(message.to_string())),
            ..Default::default()
        })
        .collect())
});
#[cfg(test)]
mod tests {
    use std::iter::zip;
    #[test]
    fn tst_regex() {
        let re = regex::Regex::new(r"-+").unwrap();
        assert!(re.is_match("---------"));
        assert!(re.is_match("-------------------"));
        let temp = "javascrpt---------it is";
        let splits: Vec<_> = re.split(temp).collect();
        let aftersplit = vec!["javascrpt", "it is"];
        for (split, after) in zip(splits, aftersplit) {
            assert_eq!(split, after);
        }
    }
    use std::process::Command;

    use tower_lsp::lsp_types::CompletionItem;

    use super::BUILDIN_COMMAND;
    #[test]
    fn tst_cmakecommand_buildin() {
        // NOTE: In case the command fails, ignore test
        let Ok(output) = Command::new("cmake").arg("--help-commands").output() else {
            return;
        };

        if let Ok(messages) = &*BUILDIN_COMMAND {
            let mut complete: Vec<CompletionItem> = vec![];
            complete.append(&mut messages.clone());
            for var in complete {
                println!(
                    "{} -- {:?} -- {:?} -- {:?}",
                    var.label, var.kind, var.insert_text, var.insert_text_format
                );
            }
        } else {
            assert!(false);
        }

        let re = regex::Regex::new(r"[z-zA-z]+\n-+").unwrap();
        let output = output.stdout;
        let temp = String::from_utf8_lossy(&output);
        let _key: Vec<_> = re.find_iter(&temp).collect();
        let splits: Vec<_> = re.split(&temp).collect();

        //for akey in key {
        //    println!("{}", akey.as_str());
        //}
        let _newsplit = &splits[1..];
        //for split in newsplit.iter() {
        //    println!("{split}");
        //}
    }
}
