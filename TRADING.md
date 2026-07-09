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

## Suggested Symbols

The following is a list of well known and stable symbols to trade.
Most symbols will not be performing well at all, that's why evaluation is critical before trading anything.

1. Precious Metals
    - SMART:NEM:USD (Newmont)
    - SMART:GOLD:USD (Barrick Gold)
    - SMART:AEM:USD (Agnico Eagle)
    - SMART:WPM:USD (Wheaton Precious Metals)
    - SMART:FNV:USD (Franco-Nevada)
    - SMART:RGLD:USD (Royal Gold)
    - SMART:PAAS:USD (Pan American Silver)
    - SMART:KGC:USD (Kinross Gold)
    - SMART:AGI:USD (Alamos Gold)
    - SMART:HMY:USD (Harmony Gold)

2. Energy & Oil
    - SMART:XOM:USD (Exxon Mobil)
    - SMART:CVX:USD (Chevron)
    - SMART:COP:USD (ConocoPhillips)
    - SMART:SLB:USD (Schlumberger)
    - SMART:EOG:USD (EOG Resources)
    - SMART:OXY:USD (Occidental Petroleum)
    - SMART:MPC:USD (Marathon Petroleum)
    - SMART:VLO:USD (Valero Energy)
    - SMART:PSX:USD (Phillips 66)
    - SMART:HAL:USD (Halliburton)
    - SMART:FANG:USD (Diamondback Energy)
    - SMART:DVN:USD (Devon Energy)

3. Tech & Semiconductors
    - SMART:NVDA:USD (Nvidia)
    - SMART:AMD:USD (AMD)
    - SMART:AVGO:USD (Broadcom)
    - SMART:QCOM:USD (Qualcomm)
    - SMART:MU:USD (Micron)
    - SMART:AMAT:USD (Applied Materials)
    - SMART:LRCX:USD (Lam Research)
    - SMART:KLAC:USD (KLA Corp)
    - SMART:MRVL:USD (Marvell Tech)
    - SMART:SNPS:USD (Synopsys)
    - SMART:CDNS:USD (Cadence)
    - SMART:CRM:USD (Salesforce)

4. Biotech & Healthcare
    - SMART:VRTX:USD (Vertex Pharma)
    - SMART:REGN:USD (Regeneron)
    - SMART:AMGN:USD (Amgen)
    - SMART:GILD:USD (Gilead Sciences)
    - SMART:DXCM:USD (DexCom)
    - SMART:ALNY:USD (Alnylam)
    - SMART:INCY:USD (Incyte)
    - SMART:PODD:USD (Insulet)
    - SMART:ILMN:USD (Illumina)
    - SMART:MRNA:USD (Moderna)
    - SMART:BNTX:USD (BioNTech)
    - SMART:NBIX:USD (Neurocrine)

5. Financials & Payments
    - SMART:JPM:USD (JPMorgan)
    - SMART:BAC:USD (Bank of America)
    - SMART:WFC:USD (Wells Fargo)
    - SMART:C:USD (Citigroup)
    - SMART:GS:USD (Goldman Sachs)
    - SMART:MS:USD (Morgan Stanley)
    - SMART:SCHW:USD (Charles Schwab)
    - SMART:AXP:USD (American Express)
    - SMART:V:USD (Visa)
    - SMART:MA:USD (Mastercard)
    - SMART:COF:USD (Capital One)

6. Industrials, Materials & Mining
    - SMART:GE:USD (GE Aerospace)
    - SMART:CAT:USD (Caterpillar)
    - SMART:DE:USD (Deere & Co)
    - SMART:HON:USD (Honeywell)
    - SMART:UNP:USD (Union Pacific)
    - SMART:CSX:USD (CSX Corp)
    - SMART:NSC:USD (Norfolk Southern)
    - SMART:RTX:USD (RTX Corp)
    - SMART:LMT:USD (Lockheed Martin)
    - SMART:NOC:USD (Northrop Grumman)
    - SMART:FCX:USD (Freeport-McMoRan)
    - SMART:NUE:USD (Nucor)
    - SMART:STLD:USD (Steel Dynamics)
    - SMART:ALB:USD (Albemarle)

7. Consumer Discretionary
    - SMART:AMZN:USD (Amazon)
    - SMART:TSLA:USD (Tesla)
    - SMART:HD:USD (Home Depot)
    - SMART:LOW:USD (Lowe's)
    - SMART:TGT:USD (Target)
    - SMART:ORLY:USD (O'Reilly Auto)
    - SMART:AZO:USD (AutoZone)
    - SMART:CMG:USD (Chipotle)
    - SMART:SBUX:USD (Starbucks)
    - SMART:NKE:USD (Nike)
    - SMART:LULU:USD (Lululemon)

To combine all these stocks into one powerful evaluation command, you run:

```
finalgo eval -r -o auto <END_DATE> SMART:NEM:USD SMART:GOLD:USD SMART:AEM:USD SMART:WPM:USD SMART:FNV:USD SMART:RGLD:USD SMART:PAAS:USD SMART:KGC:USD SMART:AGI:USD SMART:HMY:USD SMART:XOM:USD SMART:CVX:USD SMART:COP:USD SMART:SLB:USD SMART:EOG:USD SMART:OXY:USD SMART:MPC:USD SMART:VLO:USD SMART:PSX:USD SMART:HAL:USD SMART:FANG:USD SMART:DVN:USD SMART:NVDA:USD SMART:AMD:USD SMART:AVGO:USD SMART:QCOM:USD SMART:MU:USD SMART:AMAT:USD SMART:LRCX:USD SMART:KLAC:USD SMART:MRVL:USD SMART:SNPS:USD SMART:CDNS:USD SMART:CRM:USD SMART:VRTX:USD SMART:REGN:USD SMART:AMGN:USD SMART:GILD:USD SMART:DXCM:USD SMART:ALNY:USD SMART:INCY:USD SMART:PODD:USD SMART:ILMN:USD SMART:MRNA:USD SMART:BNTX:USD SMART:NBIX:USD SMART:JPM:USD SMART:BAC:USD SMART:WFC:USD SMART:C:USD SMART:GS:USD SMART:MS:USD SMART:SCHW:USD SMART:AXP:USD SMART:V:USD SMART:MA:USD SMART:COF:USD SMART:GE:USD SMART:CAT:USD SMART:DE:USD SMART:HON:USD SMART:UNP:USD SMART:CSX:USD SMART:NSC:USD SMART:RTX:USD SMART:LMT:USD SMART:NOC:USD SMART:FCX:USD SMART:NUE:USD SMART:STLD:USD SMART:ALB:USD SMART:AMZN:USD SMART:TSLA:USD SMART:HD:USD SMART:LOW:USD SMART:TGT:USD SMART:ORLY:USD SMART:AZO:USD SMART:CMG:USD SMART:SBUX:USD SMART:NKE:USD SMART:LULU:USD
```
