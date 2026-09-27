# Trading Guide

## Setup

The algorithm isn't fit for a single stock or simply letting it run on some symbols.
I recommend to evaluate it on multiple stock symbols periodically to get the best results. Ideally every month to catch
new Market Conditions.

## Ranked Evaluation

You can rank all these stocks by running the `eval` command with the `-r` flag.

To keep track of evaluation runs, you can also use the `-o` flag to specify an output path to write the result as JSON
to. You may use `-o auto` to automatically generate an output path.

## Trading

To start actually trading, you can use the `trade` command.
The command will automatically select the latest JSON file from the `eval` directory if no data path is specified.

Ideally, you only need to run:

````shell
finalgo trade <TARGET> 
````

where target is the current date or any other end date you want to use data from.

The returned trading data will be valid for `TARGEt + HORIZON` days.

Position size will automatically be calculated based on risk and account balance and orders will be placed via the IBKR
API.

## Suggested Symbols

The following is a list of well known and stable symbols to trade.
Most symbols will not be performing well at all, that's why evaluation is critical before trading anything.

I've evaluated a few tickers and this is the command with the best I could find (so far):

```
./finalgo eval -o auto -r 28.<END_DATE> SMART:LI:USD SMART:KGC:USD SMART:LYFT:USD SMART:WBD:USD SMART:AU:USD SMART:VTRS:USD SMART:AA:USD SMART:HOOD:USD SMART:SM:USD SMART:PLTR:USD SMART:MARA:USD SMART:RIOT:USD SMART:CLSK:USD SMART:IREN:USD SMART:WULF:USD SMART:CORZ:USD SMART:EXEL:USD SMART:CRSP:USD SMART:NTLA:USD SMART:HALO:USD SMART:PCVX:USD SMART:RXRX:USD SMART:VKTX:USD SMART:EDIT:USD SMART:RPRX:USD SMART:S:USD SMART:PATH:USD SMART:GTLB:USD SMART:OKTA:USD SMART:U:USD SMART:RBLX:USD SMART:RKLB:USD SMART:ASTS:USD SMART:ACHR:USD SMART:JOBY:USD SMART:UUUU:USD SMART:HBM:USD SMART:ERO:USD SMART:MP:USD SMART:NEXA:USD SMART:DNN:USD SMART:MGY:USD SMART:KOS:USD SMART:TALO:USD SMART:NU:USD SMART:RIG:USD SMART:PBR:USD SMART:CHWY:USD SMART:DKNG:USD SMART:SOFI:USD
```
