use std::collections::HashMap;
use std::iter::zip;
use std::process::Command;
use std::sync::LazyLock;
use std::path::PathBuf;
use std::fs;
use std::io::Write;

/// builtin Commands and vars
use anyhow::Result;
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, Documentation, InsertTextFormat};

use crate::languageserver::to_use_snippet;

/// NOTE:
/// This file implements a very fast zero-copy cache using rkyv + memmap2.
/// On cache hit we avoid parsing the cmake help output at all by loading
/// already-parsed CompletionItems from an rkyv binary blob mapped into memory.
///
/// Cargo.toml changes required:
/// - rkyv = "0.7"
/// - memmap2 = "0.5"
/// - optionally: sha2 = "0.10" if you want hashes later
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use rkyv::ser::serializers::AllocSerializer;
use rkyv::ser::Serializer;
use rkyv::de::deserializers::AllocDeserializer;
use memmap2::MmapOptions;

fn shorter_var(arg: &str) -> String {
    let mut shorter = arg.to_string();
    let args = arg.split('\n').next().unwrap_or("");
    if args.len() > 20 {
        shorter = format!("{}...", &args[0..20]);
    }
    if shorter.contains(' ') {
        shorter = format!("(arg_type: <{shorter}>)");
    }
    shorter = format!("<{shorter}>");
    shorter
}

fn handle_sharp_bracket(arg: &str) -> &str {
    let left_unique = arg.starts_with("<");
    let right_unique = arg.ends_with(">");
    match (left_unique, right_unique) {
        (true, true) => &arg[1..arg.len() - 1],
        (true, false) => &arg[1..],
        (false, true) => &arg[..arg.len() - 1],
        (false, false) => arg,
    }
}

fn handle_square_bracket(arg: &str) -> &str {
    let left_unique = arg.starts_with("[");
    let right_unique = arg.ends_with("]");
    match (left_unique, right_unique) {
        (true, true) => &arg[1..arg.len() - 1],
        (true, false) => &arg[1..],
        (false, true) => &arg[..arg.len() - 1],
        (false, false) => arg,
    }
}

static SNIPPET_GEN_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"<[^>]+>").unwrap());

fn convert_to_lsp_snippet(input: &str) -> String {
    let mut result = String::new();
    let mut last_pos = 0; // Keep track of the last match position
    let mut i = 1;
    for caps in SNIPPET_GEN_REGEX.captures_iter(input) {
        if let Some(matched) = caps.get(0) {
            let var_name_pre = matched.as_str(); // Extract captured variable
            let var_name_pre2 = handle_sharp_bracket(var_name_pre);
            let var_name_pre3 = handle_square_bracket(var_name_pre2);
            let var_name = shorter_var(var_name_pre3);

            // Add text before the match
            result.push_str(&input[last_pos..matched.start()]);

            // Replace the variable
            result.push_str(&format!("${{{i}:{var_name}}}"));
            // Update last position to after this match
            last_pos = matched.end();
            i += 1;
        }
    }

    // Add remaining part of the string
    result.push_str(&input[last_pos..]);

    result
}

#[test]
fn tst_convert_to_lsp_snippet() {
    let snippet_example = r#"define_property(<GLOBAL | DIRECTORY | TARGET | SOURCE |
                  TEST | VARIABLE | CACHED_VARIABLE>
                  PROPERTY <name> [INHERITED]
                  [BRIEF_DOCS <brief-doc> [docs...]]
                  [FULL_DOCS <full-doc> [docs...]]
                  [INITIALIZE_FROM_VARIABLE <variable>])"#;
    let snippet_result = convert_to_lsp_snippet(snippet_example);
    let snippet_target = r#"define_property(${1:<(arg_type: <GLOBAL | DIRECTORY |...>)}
                  PROPERTY ${2:<name>} [INHERITED]
                  [BRIEF_DOCS ${3:<brief-doc>} [docs...]]
                  [FULL_DOCS ${4:<full-doc>} [docs...]]
                  [INITIALIZE_FROM_VARIABLE ${5:<variable>}])"#;
    assert_eq!(snippet_result, snippet_target);
}

fn gen_builtin_commands(raw_info: &str) -> Result<Vec<CompletionItem>> {
    let re = regex::Regex::new(r"[a-zA-z]+\n-+").unwrap();
    let keys: Vec<_> = re
        .find_iter(raw_info)
        .map(|message| {
            let temp: Vec<&str> = message.as_str().split('\n').collect();
            temp[0]
        })
        .collect();
    let contents: Vec<_> = re.split(raw_info).collect();
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

    let client_support_snippet = to_use_snippet();

    Ok(completes
        .iter()
        .map(|(akey, message)| {
            let mut insert_text_format = InsertTextFormat::PLAIN_TEXT;
            let mut insert_text = akey.to_string();
            let mut detail = "Function".to_string();
            let s = format!(r"\n\s+(?P<signature>{akey}\([^)]*\))");
            let r_match_signature = regex::Regex::new(s.as_str()).unwrap();

            // snippets only work for lower case for now...
            if client_support_snippet
                && insert_text
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == '_')
            {
                insert_text = match r_match_signature.captures(message) {
                    Some(m) => {
                        insert_text_format = InsertTextFormat::SNIPPET;
                        detail += " (Snippet)";
                        convert_to_lsp_snippet(m.name("signature").unwrap().as_str())
                    }
                    _ => akey.to_string(),
                }
            }

            CompletionItem {
                label: akey.to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(detail),
                documentation: Some(Documentation::String(message.trim().to_string())),
                insert_text: Some(insert_text),
                insert_text_format: Some(insert_text_format),
                ..Default::default()
            }
        })
        .collect())
}

fn gen_builtin_variables(raw_info: &str) -> Result<Vec<CompletionItem>> {
    let re = regex::Regex::new(r"[z-zA-z]+\n-+").unwrap();
    let key: Vec<_> = re
        .find_iter(raw_info)
        .map(|message| {
            let temp: Vec<&str> = message.as_str().split('\n').collect();
            temp[0]
        })
        .collect();
    let content: Vec<_> = re.split(raw_info).collect();
    let context = &content[1..];
    Ok(zip(key, context)
        .map(|(akey, message)| CompletionItem {
            label: akey.to_string(),
            kind: Some(CompletionItemKind::VARIABLE),
            detail: Some("Variable".to_string()),
            documentation: Some(Documentation::String(message.trim().to_string())),
            ..Default::default()
        })
        .collect())
}

fn gen_builtin_modules(raw_info: &str) -> Result<Vec<CompletionItem>> {
    let re = regex::Regex::new(r"[z-zA-z]+\n-+").unwrap();
    let key: Vec<_> = re
        .find_iter(raw_info)
        .map(|message| {
            let temp: Vec<&str> = message.as_str().split('\n').collect();
            temp[0]
        })
        .collect();
    let content: Vec<_> = re.split(raw_info).collect();
    let context = &content[1..];
    Ok(zip(key, context)
        .map(|(akey, message)| CompletionItem {
            label: akey.to_string(),
            kind: Some(CompletionItemKind::MODULE),
            detail: Some("Module".to_string()),
            documentation: Some(Documentation::String(message.trim().to_string())),
            ..Default::default()
        })
        .collect())
}

/// Helper: decide where to place shared cache files
fn cache_dir() -> PathBuf {
    // Prefer /dev/shm on unix systems if available for in-memory fs backing
    #[cfg(unix)]
    {
        let dev_shm = PathBuf::from("/dev/shm");
        if dev_shm.is_dir() && fs::metadata(&dev_shm).is_ok() {
            return dev_shm;
        }
    }
    // Fallback to OS temp dir
    std::env::temp_dir()
}

/// Write atomically to a file in the same directory (create tmp then rename)
fn atomic_write(path: &PathBuf, contents: &[u8]) -> std::io::Result<()> {
    let mut tmp = path.clone();
    // Use a .tmp suffix to avoid clobbering existing file in case of failure
    tmp.set_extension("tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(contents)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

/// rkyv-serializable compact representation of a CompletionItem.
///
/// We keep only the fields we need to reconstruct a tower_lsp::CompletionItem.
#[derive(Archive, RkyvSerialize, RkyvDeserialize)]
#[archive_attr(derive(Debug))]
struct CachedCompletionItem {
    label: String,
    kind: u8, // 0 = Text/Unknown, 1 = Function, 2 = Variable, 3 = Module
    detail: Option<String>,
    documentation: Option<String>,
    insert_text: Option<String>,
    insert_text_format: u8, // 0 = PlainText, 1 = Snippet
}

impl From<&CompletionItem> for CachedCompletionItem {
    fn from(ci: &CompletionItem) -> Self {
        let kind = match ci.kind {
            Some(CompletionItemKind::FUNCTION) => 1,
            Some(CompletionItemKind::VARIABLE) => 2,
            Some(CompletionItemKind::MODULE) => 3,
            _ => 0,
        };
        let insert_text_format = match ci.insert_text_format {
            Some(InsertTextFormat::SNIPPET) => 1,
            _ => 0,
        };
        let documentation = ci
            .documentation
            .as_ref()
            .and_then(|d| match d {
                Documentation::String(s) => Some(s.clone()),
                _ => None,
            });
        CachedCompletionItem {
            label: ci.label.clone(),
            kind,
            detail: ci.detail.clone(),
            documentation,
            insert_text: ci.insert_text.clone(),
            insert_text_format,
        }
    }
}

impl CachedCompletionItem {
    fn into_completion_item(self) -> CompletionItem {
        let kind = match self.kind {
            1 => Some(CompletionItemKind::FUNCTION),
            2 => Some(CompletionItemKind::VARIABLE),
            3 => Some(CompletionItemKind::MODULE),
            _ => None,
        };
        let insert_text_format = match self.insert_text_format {
            1 => Some(InsertTextFormat::SNIPPET),
            _ => Some(InsertTextFormat::PLAIN_TEXT),
        };
        CompletionItem {
            label: self.label,
            kind,
            detail: self.detail,
            documentation: self.documentation.map(Documentation::String),
            insert_text: self.insert_text,
            insert_text_format,
            ..Default::default()
        }
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Default)]
#[archive_attr(derive(Debug))]
struct FastCache {
    version: String,
    commands: Option<Vec<CachedCompletionItem>>, 
    variables: Option<Vec<CachedCompletionItem>>, 
    modules: Option<Vec<CachedCompletionItem>>, 
}

impl FastCache {
    fn load_rkyv(path: &PathBuf) -> Option<Self> {
        let file = fs::File::open(path).ok()?;
        let mmap = unsafe { MmapOptions::new().map(&file).ok()? };
        let bytes = &mmap[..];
        // Validate archived root before using unsafe archived_root
        if rkyv::check_archived_root::<FastCache>(bytes).is_ok() {
            let archived = unsafe { rkyv::archived_root::<FastCache>(bytes) };
            let mut des = AllocDeserializer::<256>::default();
            match archived.deserialize(&mut des) {
                Ok(cache) => Some(cache),
                Err(e) => {
                    tracing::warn!("Failed to deserialize rkyv cache: {e}");
                    None
                }
            }
        } else {
            tracing::warn!("rkyv cache failed validation");
            None
        }
    }

    fn save_rkyv(&self, path: &PathBuf) -> std::io::Result<()> {
        let mut serializer = AllocSerializer::<256>::default();
        serializer
            .serialize_value(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{e}")))?;
        let bytes = serializer.into_inner();
        atomic_write(path, &bytes)
    }
}

/// Helper: get current cmake --version output as String (empty on failure)
fn current_cmake_version() -> String {
    match Command::new("cmake").arg("--version").output() {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).to_string(),
        _ => String::new(),
    }
}

enum HelpKind {
    Commands,
    Variables,
    Modules,
}

impl HelpKind {
    fn key(&self) -> &'static str {
        match self {
            HelpKind::Commands => "commands",
            HelpKind::Variables => "variables",
            HelpKind::Modules => "modules",
        }
    }

    fn cmake_arg(&self) -> &'static str {
        match self {
            HelpKind::Commands => "--help-commands",
            HelpKind::Variables => "--help-variables",
            HelpKind::Modules => "--help-modules",
        }
    }
}

fn kind_key_for_log(kind: &HelpKind) -> &'static str {
    match kind {
        HelpKind::Commands => "COMMAND",
        HelpKind::Variables => "VARIABLE",
        HelpKind::Modules => "MODULE",
    }
}

/// Generic helper which implements the rkyv + memmap cache
/// and returns parsed CompletionItems via the provided parser function.
///
/// Behavior:
/// - Try to load the rkyv cache file (single file: neocmakelsp_cmake_rkyv_cache).
/// - If cache.version matches `cmake --version` and the requested field is present,
///   return the cached parsed CompletionItems (zero-copy load + fast deserialization).
/// - Otherwise run the appropriate `cmake --help-*` command, parse the raw help text,
///   convert parsed CompletionItems into CachedCompletionItem and store them into the rkyv
///   cache (atomic write), then return parsed items.
fn ensure_cached_help_rkyv(
    kind: HelpKind,
    parser: impl Fn(&str) -> Result<Vec<CompletionItem>>, 
) -> Result<Vec<CompletionItem>> {
    let cache_path = cache_dir().join("neocmakelsp_cmake_rkyv_cache.bin");

    // Get current version (best-effort)
    let current_version = current_cmake_version();

    // Try loading existing rkyv cache
    let mut cache = FastCache::load_rkyv(&cache_path).unwrap_or_default();

    // If version matches and requested field present, return items from cache
    let cached_field_opt: Option<Vec<CompletionItem>> = match kind {
        HelpKind::Commands => cache
            .commands
            .as_ref()
            .map(|vec| vec.iter().cloned().map(|c| c.into_completion_item()).collect()),
        HelpKind::Variables => cache
            .variables
            .as_ref()
            .map(|vec| vec.iter().cloned().map(|c| c.into_completion_item()).collect()),
        HelpKind::Modules => cache
            .modules
            .as_ref()
            .map(|vec| vec.iter().cloned().map(|c| c.into_completion_item()).collect()),
    };

    if !current_version.is_empty() && cache.version == current_version {
        if let Some(items) = cached_field_opt {
            tracing::info!(
                "BUILTIN_{}: using rkyv cached cmake help (version match)",
                kind_key_for_log(&kind)
            );
            return Ok(items);
        }
    }

    // Cache miss or version mismatch -> generate fresh help output
    tracing::info!(
        "BUILTIN_{}: regenerating cmake help ({})",
        kind_key_for_log(&kind),
        kind.cmake_arg()
    );
    let output = Command::new("cmake").arg(kind.cmake_arg()).output()?;
    let temp = String::from_utf8_lossy(&output.stdout).to_string();

    // Parse the fresh help output (expensive)
    let parsed = parser(&temp)?;

    // Convert parsed CompletionItems into cached form
    let cached_vec: Vec<CachedCompletionItem> =
        parsed.iter().map(|ci| CachedCompletionItem::from(ci)).collect();

    // Update cache
    if !current_version.is_empty() {
        cache.version = current_version.clone();
    }
    match kind {
        HelpKind::Commands => cache.commands = Some(cached_vec),
        HelpKind::Variables => cache.variables = Some(cached_vec),
        HelpKind::Modules => cache.modules = Some(cached_vec),
    }

    // Save cache (best-effort)
    if let Err(e) = cache.save_rkyv(&cache_path) {
        tracing::warn!("Failed to write rkyv cmake help cache: {e}");
    }

    // Convert parsed into owned CompletionItems and return
    Ok(parsed)
}

/// CMake builtin commands (now using rkyv+memmap cache)
pub static BUILTIN_COMMAND: LazyLock<Result<Vec<CompletionItem>>> = LazyLock::new(|| {
    ensure_cached_help_rkyv(HelpKind::Commands, gen_builtin_commands)
});

/// cmake builtin vars (now using rkyv+memmap cache)
pub static BUILTIN_VARIABLE: LazyLock<Result<Vec<CompletionItem>>> = LazyLock::new(|| {
    ensure_cached_help_rkyv(HelpKind::Variables, gen_builtin_variables)
});

/// Cmake builtin modules (now using rkyv+memmap cache)
pub static BUILTIN_MODULE: LazyLock<Result<Vec<CompletionItem>>> = LazyLock::new(|| {
    ensure_cached_help_rkyv(HelpKind::Modules, gen_builtin_modules)
});

#[cfg(test)]
mod tests {
    use std::iter::zip;

    use super::gen_builtin_commands;
    use crate::complete::builtin::{gen_builtin_modules, gen_builtin_variables};
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

    #[test]
    fn tst_cmake_command_builtin() {
        // NOTE: In case the command fails, ignore test
        let output = include_str!("../../assets_for_test/cmake_help_commands.txt");

        let output = gen_builtin_commands(output);

        assert!(output.is_ok());
    }

    #[test]
    fn tst_cmake_variables_builtin() {
        // NOTE: In case the command fails, ignore test
        let output = include_str!("../../assets_for_test/cmake_help_variables.txt");

        let output = gen_builtin_variables(output);

        assert!(output.is_ok());
    }

    #[test]
    fn tst_cmake_modules_builtin() {
        // NOTE: In case the command fails, ignore test
        let output = include_str!("../../assets_for_test/cmake_help_commands.txt");

        let output = gen_builtin_modules(output);

        assert!(output.is_ok());
    }
}