use std::net::Ipv4Addr;
use std::path::PathBuf;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use dashmap::DashMap;
use tower_lsp::{Client, LspService, Server};
mod treesitter_nodetypes;

use tokio::net::TcpListener;
use treesitter_nodetypes as CMakeNodeKinds;
mod ast;
mod cli;
mod complete;
mod consts;
mod document_link;
mod fileapi;
mod filewatcher;
mod gammar;
mod hover;
mod jump;
mod languageserver;
mod quick_fix;
mod rename;
mod scansubs;
mod semantic_token;
mod utils;
use std::sync::OnceLock;

use tower_lsp::lsp_types::Uri;

use crate::cli::{Cli, Command};

#[derive(Debug)]
struct BackendInitInfo {
    pub scan_cmake_in_package: bool,
}

impl Default for BackendInitInfo {
    fn default() -> Self {
        Self {
            scan_cmake_in_package: true,
        }
    }
}

#[derive(Debug)]
struct Backend {
    client: Client,
    documents: DashMap<Uri, String>,
    /// Storage the message of buffers
    init_info: OnceLock<BackendInitInfo>,
    root_path: OnceLock<Option<PathBuf>>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: DashMap::new(),
            init_info: OnceLock::new(),
            root_path: OnceLock::new(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    clap_complete::CompleteEnv::with_factory(Cli::command)
        .completer(env!("CARGO_BIN_NAME"))
        .complete();

    let args = Cli::parse();

    let log = tracing_subscriber::fmt();
    if matches!(args.command, Command::Stdio) {
        // NOTE: `stdio` is used for the language server protocol, so we need to log to `stderr`.
        // Most editors can't handle ANSI escape codes in their logfiles.
        log.with_writer(std::io::stderr).with_ansi(false).init();
    } else {
        log.init();
    }

    if args.debug {
        // Safety: Calling raise is safe as long as the signal number is valid.
        // SIGSTOP (19) is a standard POSIX signal.
        unsafe {
            libc::raise(libc::SIGSTOP);
        }
    }

    match args.command {
        Command::Stdio => {
            let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
            let (service, socket) = LspService::new(Backend::new);
            Server::new(stdin, stdout, socket).serve(service).await;
        }
        Command::Tcp { port } => {
            let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await?;
            let (stream, _) = listener.accept().await?;
            let (read, write) = tokio::io::split(stream);
            let (service, socket) = LspService::new(Backend::new);
            Server::new(read, write, socket).serve(service).await;
        }
    }

    Ok(())
}
