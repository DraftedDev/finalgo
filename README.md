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

### Running

You can run the algorithm with the `run` command:

```bash
run <end-date> <ticker>
```

For example:

```bash
run 01.06.2026 SPY
```

### Evaluating

Evaluating is done using the `eval` command:

```bash
eval <end-date> <samples> <ticker1> <ticker2> <ticker3> <...>
```
