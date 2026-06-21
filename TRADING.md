# Trading Guide

## Setup

The algorithm isn't fit for a single stock or simply letting it run on some symbols.
I recommend to evaluate it on multiple stock symbols periodically to get the best results.

## Evaluating

If following this approach of periodically evaluating the algorithm,
I recommend doing that once a month to catch new market conditions.

## Target Stocks

Ideal target stocks should have long-term short and long trends and
not be in a constant bullish or bearish state.

**A list of ideal stocks:**

- `GDX`: Gold Miners
- `SIL`: Silver Miners
- `COPX`: Copper Miners
- `XLE`: Energy Select Sector
- `USO`: United Stats Oil Fund
- `DBA`: Invesco DB Agriculture
- `HYG`: High Yield Corporate Bond
- `LQD`: Investment Grade Corp Bond
- `IEF`: 7-10 year Treasury
- `EMB`: Emerging Market Bonds
- `UUP`: Invesco US Dollar Index
- `FXE`: Invesco Euro Index
- `FXB`: Invesco British Pound Index
- `FXY`: Invesco Japanese Yen Index
- `FXI`: Invesco China Index
- `VNQ`: Vanguard Real Estate
- `XLB`: Materials Select Sector
- `IWM`: Russel 2000 Small Cap
- `XBI`: SPDR S&P Biotech
- `EEM`: Emerging Markets
- `EWZ`: Brazil ETF

## Running

You can rank all these stocks by running the `eval` command with the `-r` flag.

To keep track of evaluation runs, you can also use the `-o` flag to specify an output path to write the result as JSON
to.

**I recommend to use this command:**

```bash
finalgo eval -o auto -r <TARGET> GDX SIL COPX XLE USO DBA HYG LQD IEF EMB UUP FXE FXB FXY FXI VNQ XLB IWM XBI EEM EWZ
```

where `<TARGET>` is the target end date (e.g. `01.01.2026`) and `-o auto` will automatically generate an output path to
write results to (`eval/<TARGET>.json`).
