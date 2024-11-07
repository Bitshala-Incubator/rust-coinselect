# rust-coinselect

A blockchain-agnostic coin selection library built in Rust.

## Technical Scope

The library will perform coin selection via various algorithms through a well-documented API. The API will be generic in nature and will not assume any Bitcoin structure or methods. It can be used for any UTXO-based blockchain.

The following algorithms will be implemented from scratch in Rust:

- Knapsack solving
- Branch and Bound
- Lowest Larger
- First-In-First-Out
- Single-Random-Draw

The library will have individual APIs for each algorithm and provide a wrapper API `select_coin()` which will perform selection via each algorithm and return the result with the least waste metric.

Other characteristics of the library:

- Well-documented code, helpful in understanding coin selection theory.
- Minimal possible dependency footprint.
- Minimum possible MSRV (Minimum Supported Rust Version).

## Community

The dev community gathers in a small corner of Discord [here](https://discord.gg/TSSAB3g4Zf) (say hello if you drop there from this readme).

Dev discussions predominantly happen via FOSS (Free and Open-Source Software) best practices, and by using GitHub as the Community Forum.

The Issues, PRs, and Discussions are where all the heavy lifting happens.