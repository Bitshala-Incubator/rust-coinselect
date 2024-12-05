# Development Guide

This guide will help you understand the technical scope of this library, and how to contribute.

## Problem Statement

Coin selection is the operation of selecting a subset of UTXOs from the wallet's UTXO set for constructing transactions. It is a fundamental and generic wallet management operation, and a standalone Rust library would be useful for various downstream wallet projects.

Coin selection is a variant of the Subset-Sum problem and can be solved via various algorithms. Finding an optimized solution depends on various conflicting goals such as minimizing transaction fees (fees per data byte) and confirmation time, avoid spending below dust limits, maximize privacy, and avoid bloating the network's UTXO set. 

## Background

The following literature describes the current state of coin selection in the Bitcoin ecosystem:

- Murch's coin selection thesis: [PDF](https://murch.one/erhardt2016coinselection.pdf)
- A Survey on Coin Selection Algorithms in UTXO-based Blockchains: [PDF](./docs/coinselectionpdf)
- Bitcoin Core's Coin selection module: [GitHub](https://github.com/bitcoin/bitcoin/blob/master/src/wallet/coinselection.cpp)
- Bcoin's Coin selector: [GitHub](https://github.com/bcoin-org/bcoin/blob/master/lib/wallet/coinselector.js)
- A rough implementation of the Lowest Larger Algorithm: [GitHub](https://github.com/Bitshala-Incubator/silent-pay/blob/main/packages/wallet/src/coin-selector.ts)
- Waste Metric Calculation: [GitHub](https://github.com/bitcoin/bitcoin/blob/baed5edeb611d949982c849461949c645f8998a7/src/wallet/coinselection.cpp#L795)

## Contributing

The list of things that you can contribute to,

- Bug fixes.
- New algorithms expanding the technical scope.
- Helper functions to improve the usability.

## Best practices

- Format using `cargo fmt`
- Linting using `clippy`
- Code coverage using `tarpaulin`
- Document well
