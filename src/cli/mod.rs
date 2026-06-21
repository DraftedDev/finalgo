use clap::Parser;

mod eval;
mod run;

pub async fn run(cli: Cli) {
    match cli.command {
        Subcommand::Run(args) => run::run(args).await,
        Subcommand::Eval(args) => eval::eval(args).await,
    }
}

/// Command-line-interface to the finalgo algorithm.
#[derive(Clone, Debug, Parser)]
pub struct Cli {
    /// The subcommand to run.
    #[command(subcommand)]
    pub command: Subcommand,
}

/// Subcommands for the finalgo interface.
#[derive(Clone, Debug, Parser)]
pub enum Subcommand {
    /// Run the interface.
    Run(RunArgs),
    /// Evaluate the algorithm with test data.
    Eval(EvalArgs),
}

/// Arguments for the run command.
#[derive(Clone, Debug, Parser)]
pub struct RunArgs {
    /// The target date to predict for.
    pub target: String,
    /// The ticker to use.
    pub ticker: String,
}

/// Arguments for the eval command.
#[derive(Clone, Debug, Parser)]
pub struct EvalArgs {
    /// Should the evaluator include statistics for every registered score.
    #[arg(long = "stats", short = 's')]
    pub stats: bool,
    /// The sample count to use.
    #[arg(long = "samples", short = 'c', default_value_t = 250)]
    pub samples: usize,
    /// Should the evaluator rank the tickers.
    #[arg(long = "rank", short = 'r')]
    pub rank: bool,
    /// If set, the JSON output will be written to the given path or if 'auto' the path is automatically generated.
    #[arg(long = "out", short = 'o')]
    pub out: Option<String>,
    /// The end date to use.
    pub end: String,
    /// The ticker to use.
    pub tickers: Vec<String>,
}
