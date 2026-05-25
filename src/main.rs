use crate::cli::{Cli, Subcommand};
use clap::Parser;

mod cli;
mod consts;
mod eval;
mod indicator;
mod interface;
mod math;
mod score;
mod utils;

fn main() {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_ansi(true)
        .with_file(false)
        .with_line_number(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .without_time()
        .init();

    tracing::info!(
        "Running finalgo v{} by Mikail Plotzky...",
        env!("CARGO_PKG_VERSION")
    );

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime")
        .block_on(async {
            match cli.command.clone() {
                Subcommand::Run(args) => cli.run(args).await,
                Subcommand::Eval(args) => cli.eval(args).await,
            }
        });
}
