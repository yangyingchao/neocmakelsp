use clap::builder::Styles;
use clap::builder::styling::{AnsiColor, Color, Style};
use clap::{Parser, Subcommand};

const STYLES: Styles = Styles::styled()
    .header(
        Style::new()
            .bold()
            .underline()
            .fg_color(Some(Color::Ansi(AnsiColor::Yellow))),
    )
    .usage(
        Style::new()
            .bold()
            .underline()
            .fg_color(Some(Color::Ansi(AnsiColor::Yellow))),
    )
    .literal(
        Style::new()
            .bold()
            .fg_color(Some(Color::Ansi(AnsiColor::Green))),
    )
    .invalid(
        Style::new()
            .bold()
            .fg_color(Some(Color::Ansi(AnsiColor::Red))),
    )
    .placeholder(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green))));

/// CMake LSP implementation based on tower-lsp and tree-sitter.
#[derive(Debug, Parser)]
#[command(version, long_about = None)]
#[command(styles = STYLES)]
#[command(propagate_version = true)]
pub(crate) struct Cli {
    #[arg(short, long)]
    pub(crate) debug: bool, // 添加 --debug 选项

    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Start the language server over terminal input/output streams.
    Stdio,

    /// Start the language server over TCP.
    Tcp {
        /// Port used for the TCP connection.
        #[arg(short, long, default_value_t = 9257)]
        port: u16,
    },
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }
}
