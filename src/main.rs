use crate::cli::{Cli, Subcommand};
use clap::Parser;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

mod cli;
mod consts;
mod data;
mod database;
mod engine;
mod eval;
mod indicator;
mod math;
mod regime;
mod score;
mod utils;

fn main() {
    let cli = Cli::parse();

    let indicatif_tracing = tracing_indicatif::IndicatifLayer::new();
    let level = std::env::var("LOG_LEVEL").unwrap_or("info".to_string());
    tracing_subscriber::registry()
        .with(
            EnvFilter::new("warn")
                .add_directive(format!("finalgo={}", level.as_str()).parse().unwrap()),
        )
        .with(
            fmt::layer()
                .with_writer(indicatif_tracing.get_stderr_writer())
                .with_ansi(true)
                .with_file(false)
                .with_line_number(false)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_target(false)
                .without_time(),
        )
        .with(indicatif_tracing)
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
