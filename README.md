# rust-coinselect

A blockchain-agnostic coin selection library built in Rust.

## Problem Statement

Coin selection is the operation of selecting a subset of UTXOs from the wallet's UTXO set for transaction building. It is a fundamental and generic wallet management operation, and a standalone Rust library would be useful for various downstream wallet projects.

Coin selection is a variant of the Subset-Sum problem and can be solved via various algorithms. Finding an optimized solution depends on various conflicting goals such as confirmation urgency, privacy, UTXO footprint management, etc.

## Background

The following literature describes the current state of coin selection in the Bitcoin ecosystem:

- Murch's coin selection thesis: [PDF](https://murch.one/erhardt2016coinselection.pdf)
- A Survey on Coin Selection Algorithms in UTXO-based Blockchains: [PDF](./docs/coinselectionpdf)
- Bitcoin Core's Coin selection module: [GitHub](https://github.com/bitcoin/bitcoin/blob/master/src/wallet/coinselection.cpp)
- Bcoin's Coin selector: [GitHub](https://github.com/bcoin-org/bcoin/blob/master/lib/wallet/coinselector.js)
- A rough implementation of the Lowest Larger Algorithm: [GitHub](https://github.com/Bitshala-Incubator/silent-pay/blob/main/src/wallet/coin-selector.ts)
- Waste Metric Calculation: [GitHub](https://github.com/bitcoin/bitcoin/blob/baed5edeb611d949982c849461949c645f8998a7/src/wallet/coinselection.cpp#L795)

## Technical Scope

The library will perform coin selection via various algorithms through a well-documented API. The API will be generic in nature and will not assume any Bitcoin structure or methods. It can be used for any UTXO-based blockchain.

The following algorithms will be implemented from scratch in Rust:

- Knapsack solving
- Branch and Bound
- Lowest Larger
- First-In-First-Out
- Single-Random-Draw

The library will have individual APIs for each algorithm and provide a wrapper API `coin_select()` which will perform selection via each algorithm and return the result with the least waste metric.

Other characteristics of the library:

- Well-documented code, helpful in understanding coin selection theory.
- Minimal possible dependency footprint.
- Minimum possible MSRV (Minimum Supported Rust Version).

## Contributing

The project is under active development by a few motivated Rusty Bitcoin devs. Contributions for features, tests, docs, and other fixes/upgrades are encouraged and welcomed. The maintainers will use the PR thread to provide quick reviews and suggestions and are generally proactive at merging good contributions.

Directions for new contributors:

- The list of [issues](https://github.com/Bitshala-Incubator/rust-coinselect/issues) is a good place to look for contributable tasks and open problems.
- Issues marked with [`good first issue`](https://github.com/Bitshala-Incubator/rust-coinselect/issues?q=is%3Aopen+is%3Aissue+label%3A%22good+first+issue%22) are good places to get started for newbie Rust/Bitcoin devs.
- The background [docs](#background) are a good place to start reading up on coin selection theory.
- Reviewing [open PRs](https://github.com/Bitshala-Incubator/rust-coinselect/pulls) is a good place to start gathering a contextual understanding of the codebase.
- Search for `TODO`s in the codebase to find in-line marked code todos and smaller improvements.

## Community

The dev community gathers in a small corner of Discord [here](https://discord.gg/TSSAB3g4Zf) (say hello if you drop there from this readme).

Dev discussions predominantly happen via FOSS (Free and Open-Source Software) best practices, and by using GitHub as the Community Forum.

The Issues, PRs, and Discussions are where all the heavy lifting happens.