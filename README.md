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

- Free Data fetching from the Interactive Brokers API.
- Bulk-fetches data to not hit the API limits.
- Complete Engine + Indicators + Scores + Metrics architecture.
- Open-Source and licensed under the [MIT-License](./LICENSE).

## Usage

The project itself is a binary and contains a CLI with different commands.

Since FinalGo uses the [Interactive Brokers](https://www.interactivebrokers.com/) API, an IBKR account is required.
Users should also install the [IB Gateway](https://www.interactivebrokers.com/en/trading/ibgateway-latest.php) Companion
App, log in and let it run in the background.

**NOTE:** By default, the paper-trading mode is permanently enabled. To enable real-life trading, you need to set the
environment variable `ALLOW_TRADING` to `1`. Proceed with caution!

### Command-Line-Interface

```
Command-line-interface to the finalgo algorithm

Usage: finalgo <COMMAND>

Commands:
  run    Run the interface
  trade  Trade with the interface
  eval   Evaluate the algorithm with test data
  help   Print this message or the help of the given subcommand(s)

Options:
  -p, --paper  Should the interface connect to IB-Gateway in paper-trading mode
  -h, --help   Print help
```

---

#### `finalyst run`

Runs the interface on the given target date and ticker.

The stock symbol must have data from `TARGET - CANDLE_LOOK_BACK (110 candles)` to `TARGET`.

Predicted output is valid for `TARGET + HORIZON (5 trading days)`.

```
Run the interface

Usage: finalgo run <TARGET> <TICKER>

Arguments:
  <TARGET>  The target date to predict for
  <TICKER>  The ticker to use
```

---

#### `finalyst trade`

Runs the interface and outputs trade results.

Uses a data file generated via ranked evaluation (`finalgo eval -r`) to select tickers.

The stock symbols inside the data file must have data from `TARGET - CANDLE_LOOK_BACK (150 candles)` to `TARGET`.

Predicted output is valid for `TARGET + HORIZON (5 trading days)`.

See [Trading Guide](./TRADING.md) for more information.

```
Trade with the interface

Usage: finalgo trade [OPTIONS] <TARGET>

Arguments:
  <TARGET>  The target date to predict for

Options:
  -d, --data <DATA>  Path to a JSON file generated via `eval -r -o <PATH>` or 'auto' to automatically find the latest file [default: auto]
```

---

#### `finalyst eval`

Evaluates the algorithm on given tickers and outputs results of various metrics.

The stock symbols must have data from `(TARGET - CANDLE_LOOK_BACK (150 candles)) * samples` to `TARGET`.

```
Evaluate the algorithm with test data

Usage: finalgo eval [OPTIONS] <END> [TICKERS]...

Arguments:
  <END>         The end date to use
  [TICKERS]...  The ticker to use

Options:
  -s, --stats              Should the evaluator include statistics for every registered score
  -c, --samples <SAMPLES>  The sample count to use [default: 250]
  -r, --rank               Should the evaluator rank the tickers
  -o, --out <OUT>          If set, the JSON output will be written to the given path or if 'auto' the path is automatically generated
```

## Real-World Usage

I recommend to first paper-trade with this algorithm.

If you want to actually use it in the real world, checkout the [Trading Guide](./TRADING.md).
