# FinAlgo - A financial trading algorithm

**This is a work-in-progress financial trading algorithm to predict market trades.**

## Introduction

I've been working on these kinds of projects for a while now.

I created an [AI](https://github.com/DraftedDev/mirada-ai) for stock market predictions
and even an RSS Feed [RAG-System](https://github.com/DraftedDev/finalyst),
but they aren't really ideal for real-life trading.

This algorithm is my best attempt yet.

It's written in Rust and uses the Alpaca Finance API for fetching market data for free.

## Features

- Free Data fetching from the Alpaca Finance API.
- Bulk-fetches data to not hit the API limits.
- Complete Engine + Indicators + Scores + Metrics architecture.
- Open-Source and licensed under the [MIT-License](./LICENSE).

## Usage

The project itself is a binary and contains a CLI with different commands.

Since FinalGo uses the Alpaca Finance API, an account and API key are required.

Store the API key in `secrets/ALPACA_KEY` and the API secret in `secrets/ALPACA_SECRET`.

Git ignores these files, so you can commit changes without needing to worry about them.

### Command-Line-Interface

```
Command-line-interface to the finalgo algorithm

Usage: finalgo <COMMAND>

Commands:
  run   Run the interface
  eval  Evaluate the algorithm with test data
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

#### `finalyst run`

Runs the interface on the given target date and ticker.

The stock symbol must have data from `TARGET - CANDLE_LOOK_BACK (610 candles)` to `TARGET`.

Predicted output is valid for `TARGET + HORIZON (5 trading days)`.

```
Usage: finalgo run <TARGET> <TICKER>

Arguments:
  <TARGET>  The target date to predict for
  <TICKER>  The ticker to use
```

#### `finalyst eval`

Evaluates the algorithm on given tickers and outputs results of various metrics.

The stock symbols must have data from `TARGET - CANDLE_LOOK_BACK (610 candles) * samples` to `TARGET`.

```
Usage: finalgo eval [OPTIONS] <END> [TICKERS]...

Arguments:
  <END>         The end date to use
  [TICKERS]...  The ticker to use

Options:
  -s, --stats              Should the evaluator include statistics for every registered score
  -c, --samples <SAMPLES>  The sample count to use [default: 250]
```

## Real-World Usage

I recommend to first paper-trade with this algorithm.

If you want to actually use it in the real world, checkout the [Trading Guide](./TRADING.md).
