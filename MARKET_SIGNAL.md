# Market signal — does the buyer exist yet? (2026-07 desk research)

The dominant risk for Aperture is not the tech — it's whether **funds actually hold, or plan to
hold, Solana Token-2022 confidential balances**. Since I can't yet talk to a fund, this is a desk
read of adoption signals. **Verdict: the market is pre-formation.** The disclosure-layer thesis is
sound, but it is early by one or two layers. Recommendation: **hold Aperture at option value, don't
invest heavily into GTM yet, and watch two specific unlock triggers.**

## What the signals say

**1. The substrate itself is not transactable on mainnet today.**
Token-2022 confidential balances shipped, but the **ZK ElGamal Proof program is feature-gated off**
on mainnet (June-2025 forged-proof incident). So confidential transfers are **not executable on
mainnet right now** — funds literally cannot use the substrate live, regardless of tooling. The
market can't form until this flips.

**2. Custody/wallet support for confidential balances is absent.**
Institutional Solana treasury infra is strong and growing — Fireblocks × Solana (Jan 2026 deep
integration, treasury OS, MPC custody), Squads as the multisig standard, used by BNY/Revolut/Galaxy.
But none of it is **confidential-balance-aware**: custody supports Solana treasuries, not encrypted
balances specifically. An institution with Fireblocks/Squads cannot easily hold a confidential
balance today. (Matches the "wallets still catching up" note from the earlier scan.)

**3. The institutional Solana holders that exist are the anti-use-case.**
The big on-chain SOL holders are **public treasury companies** (e.g. Forward Industries ~7M SOL,
$1.65B PIPE) and hedge funds via SOL ETFs. These are *disclosure-mandated* by construction — they
hold publicly. They are the opposite of the "hide my position size" buyer. Real crypto-native funds
that would want amount-hiding exist, but there's no visible signal of them holding confidential
balances yet.

**4. Demand signal is real but sits in the primitive/consumer layer, and actual usage is tiny.**
Arcium/Umbra is the proxy for confidential-DeFi demand: Umbra's **$155M ICO** (Oct 2025, largest
Solana ICO, 10,500 participants), Arcium mainnet alpha at ~200k computations/day, 25+ projects, and
**CSPL launching Q1 2026** explicitly pitched to bring "institutional finance on-chain." But Umbra's
private mainnet is gated to **100 users/week at a $500 deposit cap** — capital interest is loud,
actual usage is tiny and still in stability testing. Demand is concentrated in the *substrate/infra*
and *consumer privacy*, not yet in *fund treasury disclosure*.

## Read

- The **thesis** (funds will want amount-privacy + selective disclosure) is directionally right and
  the infra momentum (CSPL, Arcium) confirms the category is forming.
- The **buyer is not transacting yet**: substrate disabled on mainnet + no confidential-aware custody
  + the visible institutional holders are disclosure-mandated. Heavy GTM now would be selling into a
  market that cannot yet execute.
- Being early at the *disclosure layer* is survivable **only** as option value — not as a
  full-investment bet, especially solo/bootstrap.

## Two triggers that flip go → invest

1. **ZK ElGamal Proof program re-enabled on mainnet** — the substrate becomes transactable. (Track
   token-2022 issue #657 / Agave changelog.)
2. **Confidential-balance support appears in institutional custody/wallets** (Fireblocks, Squads,
   or a wallet-as-a-service) — funds can actually hold encrypted balances. This is the demand-side
   unlock and the stronger of the two signals.

**Leading indicator to watch:** **Arcium CSPL** institutional traction. If CSPL gets real fund/
treasury usage, that is simultaneously (a) proof the buyer exists and (b) a substrate Aperture can
ride via the `cspl` adapter — Aperture rides the winner rather than betting on Token-2022 alone.

## Posture recommendation

Keep the built stack as a validated, ready option (spike + core + receipts + flow all green). Do
**not** run a heavy design-partner campaign into a non-transactable market. Instead: monitor the two
triggers, keep one warm exploratory conversation open if a confidential-native fund surfaces, and
re-engage GTM the moment trigger #1 or #2 fires.

## Sources

- [Confidential transfers on Solana — Everstake](https://everstake.one/resources/blog/blockchain-privacy-confidential-transfers)
- [Confidential Balances live — QuickNode](https://blog.quicknode.com/confidential-balance-token-extensions-on-solana/)
- [Solana × Fireblocks institutional treasury](https://solana.com/news/solana-fireblocks-institutional-treasury-infrastructure)
- [Best Solana trading platforms (Squads/Fireblocks custody) — Definitive](https://www.definitive.fi/blog/best-solana-trading-platforms)
- [5 largest Solana treasury firms — Yahoo Finance](https://finance.yahoo.com/news/5-largest-publicly-traded-solana-132838567.html)
- [Arcium CSPL / Umbra keynote — Solana Compass](https://solanacompass.com/learn/breakpoint-25/keynote-arcium-yannik-schrade)
- [Arcium mainnet alpha / Umbra — The Block](https://www.theblock.co/post/387564/arcium-launches-privacy-preserving-mainnet-alpha-on-solana-as-umbra-debuts-shielded-finance-layer)
