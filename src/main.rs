use clap::Parser;
use std::path::PathBuf;

mod ast;
mod complete;
mod consts;
mod filewatcher;
mod formatting;
mod grammar;
mod jump;
mod languageserver;
mod scansubs;
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

    scan_handle: Option<tokio::task::JoinHandle<()>>,
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
                scan_handle: None,
            }))
    });

    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_ansi(false)
        .with_writer(std::io::stderr)
        .init();

    server.run_buffered(input, output).await.unwrap()
}

#[derive(Parser)]
#[command(long_about = None, about = "CMake Lsp implementation based on async-lsp and Tree-sitter",
    arg_required_else_help = true,    author = "Cris",
    version )]
struct Cli {
    #[arg(long = "stdio", help = "run with stdio (default and only option...)")]
    stdio: bool,
}

fn parse_args<T, S>(args: T) -> Cli
where
    T: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args = args.into_iter().map(|x| x.into()).collect::<Vec<String>>();
    // backward compatible. support `emacs-lsp-booster server_cmd args...` directly
    if args.len() > 1 && !args[1].starts_with('-') && !args.contains(&"--".into()) {
        let mut fake_args = vec![args[0].clone(), "--".into()];
        fake_args.extend_from_slice(&args[1..]);
        Cli::parse_from(fake_args)
    } else {
        Cli::parse_from(args)
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let _cli = parse_args(std::env::args());

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

#[test]
fn test_parse_args() {
    let cli = parse_args(vec!["neocmakelsp", "--stdio"]);
    assert_eq!(cli.stdio, true);
    // assert_eq!(cli.verbose.log_level_filter(), log::LevelFilter::Info);
}
