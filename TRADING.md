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

1. **Tech, Fintech & Internet**
    - SMART:INTC:USD (Intel)
    - SMART:SOFI:USD (SoFi Technologies)
    - SMART:PLTR:USD (Palantir Technologies)
    - SMART:HOOD:USD (Robinhood Markets)
    - SMART:SNAP:USD (Snap Inc.)
    - SMART:PINS:USD (Pinterest)
    - SMART:ROKU:USD (Roku)
    - SMART:LYFT:USD (Lyft)
    - SMART:PYPL:USD (PayPal Holdings)
    - SMART:AFRM:USD (Affirm Holdings)
    - SMART:UPST:USD (Upstart Holdings)
    - SMART:DKNG:USD (DraftKings)
    - SMART:CHWY:USD (Chewy)
    - SMART:W:USD (Wayfair)
    - SMART:ETSY:USD (Etsy)

2. **Auto, EV & Travel**
    - SMART:F:USD (Ford Motor)
    - SMART:GM:USD (General Motors)
    - SMART:STLA:USD (Stellantis)
    - SMART:RIVN:USD (Rivian Automotive)
    - SMART:NIO:USD (NIO)
    - SMART:XPEV:USD (XPeng)
    - SMART:LI:USD (Li Auto)
    - SMART:CCL:USD (Carnival Corp.)
    - SMART:NCLH:USD (Norwegian Cruise Line Holdings)
    - SMART:DAL:USD (Delta Air Lines)
    - SMART:UAL:USD (United Airlines Holdings)
    - SMART:AAL:USD (American Airlines Group)
    - SMART:LUV:USD (Southwest Airlines)
    - SMART:JBLU:USD (JetBlue Airways)
    - SMART:ALK:USD (Alaska Air Group)

3. **Energy, Oil, Gas & Uranium**
    - SMART:OXY:USD (Occidental Petroleum)
    - SMART:HAL:USD (Halliburton)
    - SMART:APA:USD (APA Corporation)
    - SMART:DVN:USD (Devon Energy)
    - SMART:PR:USD (Permian Resources)
    - SMART:AR:USD (Antero Resources)
    - SMART:RRC:USD (Range Resources)
    - SMART:EQT:USD (EQT Corporation)
    - SMART:CNX:USD (CNX Resources)
    - SMART:CHRD:USD (Chord Energy)
    - SMART:SM:USD (SM Energy)
    - SMART:MTDR:USD (Matador Resources)
    - SMART:CCJ:USD (Cameco - Uranium)
    - SMART:LEU:USD (Centrus Energy - Uranium)

4. **Metals, Mining & Materials**
    - SMART:VALE:USD (Vale S.A.)
    - SMART:TECK:USD (Teck Resources)
    - SMART:KGC:USD (Kinross Gold)
    - SMART:GOLD:USD (Barrick Gold)
    - SMART:PAAS:USD (Pan American Silver)
    - SMART:AGI:USD (Alamos Gold)
    - SMART:AU:USD (AngloGold Ashanti)
    - SMART:GFI:USD (Gold Fields)
    - SMART:HMY:USD (Harmony Gold)
    - SMART:CLF:USD (Cleveland-Cliffs)
    - SMART:AA:USD (Alcoa)
    - SMART:FCX:USD (Freeport-McMoRan)
    - SMART:RIO:USD (Rio Tinto Group)
    - SMART:BHP:USD (BHP Group)
    - SMART:MT:USD (ArcelorMittal)
    - SMART:PKX:USD (POSCO Holdings)
    - SMART:SID:USD (Companhia Siderúrgica Nacional)
    - SMART:HXL:USD (Hexcel)

5. **Healthcare & Biotech**
    - SMART:PFE:USD (Pfizer)
    - SMART:MRNA:USD (Moderna)
    - SMART:BNTX:USD (BioNTech)
    - SMART:OGN:USD (Organon)
    - SMART:VTRS:USD (Viatris)
    - SMART:PRGO:USD (Perrigo)
    - SMART:HSIC:USD (Henry Schein)
    - SMART:XRAY:USD (Dentsply Sirona)
    - SMART:CNC:USD (Centene)
    - SMART:CVS:USD (CVS Health)
    - SMART:UHS:USD (Universal Health Services)
    - SMART:THC:USD (Tenet Healthcare)

6. **Financials, Banks & Insurance**
    - SMART:BAC:USD (Bank of America)
    - SMART:WFC:USD (Wells Fargo)
    - SMART:C:USD (Citigroup)
    - SMART:SCHW:USD (Charles Schwab)
    - SMART:COF:USD (Capital One)
    - SMART:USB:USD (U.S. Bancorp)
    - SMART:TFC:USD (Truist Financial)
    - SMART:FITB:USD (Fifth Third Bancorp)
    - SMART:CFG:USD (Citizens Financial Group)
    - SMART:KEY:USD (KeyCorp)
    - SMART:RF:USD (Regions Financial)
    - SMART:HBAN:USD (Huntington Bancshares)
    - SMART:ZION:USD (Zions Bancorporation)
    - SMART:ALLY:USD (Ally Financial)
    - SMART:SYF:USD (Synchrony Financial)
    - SMART:AIG:USD (American International Group)
    - SMART:MET:USD (MetLife)
    - SMART:PRU:USD (Prudential Financial)
    - SMART:AFL:USD (Aflac)
    - SMART:UNM:USD (Unum Group)
    - SMART:LNC:USD (Lincoln National)

7. **Telecom, Media & Utilities**
    - SMART:T:USD (AT&T)
    - SMART:VZ:USD (Verizon Communications)
    - SMART:CHTR:USD (Charter Communications)
    - SMART:CMCSA:USD (Comcast)
    - SMART:DIS:USD (The Walt Disney Company)
    - SMART:WBD:USD (Warner Bros. Discovery)
    - SMART:FOX:USD (Fox Corporation Class B)
    - SMART:FOXA:USD (Fox Corporation Class A)
    - SMART:SPOT:USD (Spotify Technology)
    - SMART:SIRI:USD (SiriusXM Holdings)
    - SMART:EXC:USD (Exelon)
    - SMART:XEL:USD (Xcel Energy)
    - SMART:PEG:USD (Public Service Enterprise Group)
    - SMART:ES:USD (Eversource Energy)
    - SMART:DTE:USD (DTE Energy)
    - SMART:CMS:USD (CMS Energy)
    - SMART:CNP:USD (CenterPoint Energy)
    - SMART:ETR:USD (Entergy)
    - SMART:FE:USD (FirstEnergy)
    - SMART:NI:USD (NiSource)
    - SMART:EVRG:USD (Evergy)
    - SMART:PNW:USD (Pinnacle West Capital)
    - SMART:PPL:USD (PPL Corporation)
    - SMART:AES:USD (The AES Corporation)
    - SMART:NRG:USD (NRG Energy)
    - SMART:TLN:USD (Talen Energy)
    - SMART:FSLR:USD (First Solar)
    - SMART:ENPH:USD (Enphase Energy)
    - SMART:RUN:USD (Sunrun)

8. **Consumer & Retail**
    - SMART:KR:USD (The Kroger Co.)
    - SMART:DG:USD (Dollar General)
    - SMART:DLTR:USD (Dollar Tree)
    - SMART:BBY:USD (Best Buy)
    - SMART:AAP:USD (Advance Auto Parts)
    - SMART:MNRO:USD (Monro, Inc.)
    - SMART:PZZA:USD (Papa John's International)
    - SMART:DIN:USD (Dine Brands Global)
    - SMART:JACK:USD (Jack in the Box)
    - SMART:WEN:USD (The Wendy's Company)
    - SMART:YUM:USD (Yum! Brands)
    - SMART:QSR:USD (Restaurant Brands International)
    - SMART:CROX:USD (Crocs, Inc.)
    - SMART:BIRK:USD (Birkenstock Holding)
    - SMART:ONON:USD (On Holding AG)
    - SMART:HAS:USD (Hasbro)
    - SMART:MAT:USD (Mattel)
    - SMART:TPR:USD (Tapestry, Inc.)
    - SMART:RL:USD (Ralph Lauren)
    - SMART:PVH:USD (PVH Corp.)
    - SMART:LB:USD (LB / Legacy Retail)
    - SMART:KMB:USD (Kimberly-Clark)
    - SMART:EL:USD (The Estée Lauder Companies)
    - SMART:CHD:USD (Church & Dwight)
    - SMART:GIS:USD (General Mills)

To combine all these stocks into one powerful evaluation command, you run:

```
./finalgo eval -o auto -r <END_DATE> SMART:INTC:USD SMART:SOFI:USD SMART:PLTR:USD SMART:HOOD:USD SMART:SNAP:USD SMART:PINS:USD SMART:ROKU:USD SMART:LYFT:USD SMART:PYPL:USD SMART:AFRM:USD SMART:UPST:USD SMART:DKNG:USD SMART:CHWY:USD SMART:W:USD SMART:ETSY:USD SMART:F:USD SMART:GM:USD SMART:STLA:USD SMART:RIVN:USD SMART:NIO:USD SMART:XPEV:USD SMART:LI:USD SMART:CCL:USD SMART:NCLH:USD SMART:DAL:USD SMART:UAL:USD SMART:AAL:USD SMART:LUV:USD SMART:JBLU:USD SMART:ALK:USD SMART:OXY:USD SMART:HAL:USD SMART:APA:USD SMART:DVN:USD SMART:PR:USD SMART:AR:USD SMART:RRC:USD SMART:EQT:USD SMART:CNX:USD SMART:CHRD:USD SMART:SM:USD SMART:MTDR:USD SMART:CCJ:USD SMART:LEU:USD SMART:VALE:USD SMART:TECK:USD SMART:KGC:USD SMART:GOLD:USD SMART:PAAS:USD SMART:AGI:USD SMART:AU:USD SMART:GFI:USD SMART:HMY:USD SMART:CLF:USD SMART:AA:USD SMART:FCX:USD SMART:RIO:USD SMART:BHP:USD SMART:MT:USD SMART:PKX:USD SMART:SID:USD SMART:HXL:USD SMART:PFE:USD SMART:MRNA:USD SMART:BNTX:USD SMART:OGN:USD SMART:VTRS:USD SMART:PRGO:USD SMART:HSIC:USD SMART:XRAY:USD SMART:CNC:USD SMART:CVS:USD SMART:UHS:USD SMART:THC:USD SMART:BAC:USD SMART:WFC:USD SMART:C:USD SMART:SCHW:USD SMART:COF:USD SMART:USB:USD SMART:TFC:USD SMART:FITB:USD SMART:CFG:USD SMART:KEY:USD SMART:RF:USD SMART:HBAN:USD SMART:ZION:USD SMART:ALLY:USD SMART:SYF:USD SMART:AIG:USD SMART:MET:USD SMART:PRU:USD SMART:AFL:USD SMART:UNM:USD SMART:LNC:USD SMART:T:USD SMART:VZ:USD SMART:CHTR:USD SMART:CMCSA:USD SMART:DIS:USD SMART:WBD:USD SMART:FOX:USD SMART:FOXA:USD SMART:SPOT:USD SMART:SIRI:USD SMART:EXC:USD SMART:XEL:USD SMART:PEG:USD SMART:ES:USD SMART:DTE:USD SMART:CMS:USD SMART:CNP:USD SMART:ETR:USD SMART:FE:USD SMART:NI:USD SMART:EVRG:USD SMART:PNW:USD SMART:PPL:USD SMART:AES:USD SMART:NRG:USD SMART:TLN:USD SMART:FSLR:USD SMART:ENPH:USD SMART:RUN:USD SMART:KR:USD SMART:DG:USD SMART:DLTR:USD SMART:BBY:USD SMART:AAP:USD SMART:MNRO:USD SMART:PZZA:USD SMART:DIN:USD SMART:JACK:USD SMART:WEN:USD SMART:YUM:USD SMART:QSR:USD SMART:CROX:USD SMART:BIRK:USD SMART:ONON:USD SMART:HAS:USD SMART:MAT:USD SMART:TPR:USD SMART:RL:USD SMART:PVH:USD SMART:LB:USD SMART:KMB:USD SMART:EL:USD SMART:CHD:USD SMART:GIS:USD
```
