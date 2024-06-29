use clap::Parser;
use consts::TREESITTER_CMAKE_LANGUAGE;
use ini::Ini;
use std::io::prelude::*;
use std::path::PathBuf;

use ignore::gitignore::{Gitignore, GitignoreBuilder};

mod ast;
mod clapargs;
mod complete;
mod config;
mod consts;
mod filewatcher;
mod formatting;
mod gammar;
mod jump;
mod languageserver;
mod scansubs;
mod search;
mod utils;

use futures::{AsyncRead, AsyncWrite};

use async_lsp::client_monitor::ClientProcessMonitorLayer;
use async_lsp::concurrency::ConcurrencyLayer;
use async_lsp::panic::CatchUnwindLayer;
use async_lsp::router::Router;
use async_lsp::server::LifecycleLayer;
use async_lsp::tracing::TracingLayer;
use async_lsp::ClientSocket;
use tower::ServiceBuilder;
use tracing::Level;

use clapargs::NeocmakeCli;

#[derive(Debug)]
struct BackendInitInfo {
    pub scan_cmake_in_package: bool,
}

/// Beckend
#[derive(Debug)]
struct Backend {
    /// client
    client: ClientSocket,

    /// Storage the message of buffers
    init_info: BackendInitInfo,
    root_path: Option<PathBuf>,
}

fn gitignore() -> Option<Gitignore> {
    let gitignore = std::path::Path::new(".gitignore");
    if !gitignore.exists() {
        return None;
    }
    let mut builder = GitignoreBuilder::new(std::env::current_dir().ok()?);
    builder.add(gitignore);
    builder.build().ok()
}

fn editconfig_setting() -> Option<(bool, u32)> {
    let editconfig_path = std::path::Path::new(".editorconfig");
    if !editconfig_path.exists() {
        return None;
    }
    let conf = Ini::load_from_file(editconfig_path).unwrap();

    let cmakesession = conf.section(Some("CMakeLists.txt"))?;

    let indent_style = cmakesession.get("indent_style").unwrap_or("space");
    let use_space = indent_style == "space";
    let indent_size = cmakesession.get("indent_size").unwrap_or("2");
    let indent_size: u32 = if use_space {
        indent_size.parse::<u32>().unwrap_or(2)
    } else {
        1
    };

    Some((use_space, indent_size))
}

async fn start_server(input: impl AsyncRead, output: impl AsyncWrite) {
    let (server, _) = async_lsp::MainLoop::new_server(|client| {
        ServiceBuilder::new()
            .layer(TracingLayer::default())
            .layer(LifecycleLayer::default())
            .layer(CatchUnwindLayer::default())
            .layer(ConcurrencyLayer::default())
            .layer(ClientProcessMonitorLayer::new(client.clone()))
            .service(Router::from_language_server(Backend {
                client,
                init_info: BackendInitInfo {
                    scan_cmake_in_package: false,
                },
                root_path: None,
            }))
    });

    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_ansi(false)
        .with_writer(std::io::stderr)
        .init();

    server.run_buffered(input, output).await.unwrap()
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = NeocmakeCli::parse();
    match args {
        NeocmakeCli::Stdio => {
            // Prefer truly asynchronous piped stdin/stdout without blocking tasks.
            #[cfg(unix)]
            let (stdin, stdout) = (
                async_lsp::stdio::PipeStdin::lock_tokio().unwrap(),
                async_lsp::stdio::PipeStdout::lock_tokio().unwrap(),
            );
            // Fallback to spawn blocking read/write otherwise.
            #[cfg(not(unix))]
            let (stdin, stdout) = (
                tokio_util::compat::TokioAsyncReadCompatExt::compat(tokio::io::stdin()),
                tokio_util::compat::TokioAsyncWriteCompatExt::compat_write(tokio::io::stdout()),
            );
            start_server(stdin, stdout).await;
        }
        NeocmakeCli::Tree { tree_path, tojson } => {
            match scansubs::get_treedir(&tree_path) {
                Some(tree) => {
                    if tojson {
                        println!("{}", serde_json::to_string(&tree).unwrap())
                    } else {
                        println!("{tree}")
                    }
                }
                None => println!("Nothing find"),
            };
        }
        NeocmakeCli::Search { package, tojson } => {
            if tojson {
                println!("{}", search::search_result_tojson(package.as_str()));
            } else {
                println!("{}", search::search_result(package.as_str()));
            }
        }
        NeocmakeCli::Format {
            format_path,
            hasoverride,
        } => {
            let (use_space, spacelen) = editconfig_setting().unwrap_or((true, 2));
            let ignorepatterns = gitignore();

            let formatpattern = |pattern: &str| {
                for filepath in glob::glob(pattern)
                    .unwrap_or_else(|_| panic!("error pattern"))
                    .flatten()
                {
                    if let Some(ref ignorepatterns) = ignorepatterns {
                        if ignorepatterns.matched(&filepath, false).is_ignore() {
                            continue;
                        }
                    }

                    let mut file = match std::fs::OpenOptions::new()
                        .read(true)
                        .write(hasoverride)
                        .open(&filepath)
                    {
                        Ok(file) => file,
                        Err(e) => {
                            println!("cannot read file {} :{e}", filepath.display());
                            continue;
                        }
                    };
                    let mut buf = String::new();
                    file.read_to_string(&mut buf).unwrap();
                    let mut parse = tree_sitter::Parser::new();
                    parse.set_language(&TREESITTER_CMAKE_LANGUAGE).unwrap();
                    match formatting::get_format_cli(&buf, spacelen, use_space) {
                        Some(context) => {
                            if hasoverride {
                                if let Err(e) = file.set_len(0) {
                                    println!("Cannot clear the file: {e}");
                                };
                                if let Err(e) = file.seek(std::io::SeekFrom::End(0)) {
                                    println!("Cannot jump to end: {e}");
                                };
                                let Ok(_) = file.write_all(context.as_bytes()) else {
                                    println!("cannot write in {}", filepath.display());
                                    continue;
                                };
                                let _ = file.flush();
                            } else {
                                println!("== Format of file {} is ==", filepath.display());
                                println!("{context}");
                                println!("== End ==");
                                println!();
                            }
                        }
                        None => {
                            println!("There is error in file: {}", filepath.display());
                        }
                    }
                }
            };
            let toformatpath = std::path::Path::new(format_path.as_str());
            if toformatpath.exists() {
                if toformatpath.is_file() {
                    let mut file = match std::fs::OpenOptions::new()
                        .read(true)
                        .write(hasoverride)
                        .open(&format_path)
                    {
                        Ok(file) => file,
                        Err(e) => {
                            println!("cannot read file {} :{e}", format_path);
                            return;
                        }
                    };
                    let mut buf = String::new();
                    file.read_to_string(&mut buf).unwrap();
                    match formatting::get_format_cli(&buf, spacelen, use_space) {
                        Some(context) => {
                            if hasoverride {
                                if let Err(e) = file.set_len(0) {
                                    println!("Cannot clear the file: {e}");
                                };
                                if let Err(e) = file.seek(std::io::SeekFrom::End(0)) {
                                    println!("Cannot jump to end: {e}");
                                };
                                let Ok(_) = file.write_all(context.as_bytes()) else {
                                    println!("cannot write in {}", format_path);
                                    return;
                                };
                                let _ = file.flush();
                            } else {
                                println!("{context}")
                            }
                        }
                        None => {
                            println!("There is error in file: {}", format_path);
                        }
                    }
                } else {
                    formatpattern(&format!("./{}/**/*.cmake", format_path));
                    formatpattern(&format!("./{}/**/CMakeLists.txt", format_path));
                }
            }
        }
    }
}
