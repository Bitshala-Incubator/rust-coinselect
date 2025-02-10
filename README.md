# rust-coinselect

A blockchain-agnostic coin selection library built in Rust.

## Technical Scope

The library performs coin selection via various algorithms through a well-documented API. The API is generic in nature and does not assume any structure or method specific to bitcoin. It is designed to be used by any UTXO-based blockchain.

The following algorithms are implemented from scratch in Rust:

- Knapsack
- Branch and Bound
- Lowest Larger
- First-In-First-Out
- Single-Random-Draw

The library has individual APIs for each algorithm. It also has a wrapper API `select_coin()` which performs selection via each algorithm and return the selection result with the least waste metric.

Bitcoin specific example is given [here](./examples/bitcoin_crate/).

An example usage is given below

```rust
use rust_coinselect::{
    types::{CoinSelectionOpt, ExcessStrategy, OutputGroup},
    selectcoin::select_coin,
};

// List of the available UTXOs
// let utxos: Vec<UTXO> = vec![<utxo1>, <utxo2>, ..., <utxon>];

// UTXOs converted to OutputGroups
let output_groups = vec![
    OutputGroup { value: 1_000_000, weight: 100, input_count: 1, creation_sequence: None },
    OutputGroup { value: 2_000_000, weight: 100, input_count: 1, creation_sequence: None },
];

let options = CoinSelectionOpt {
    target_value: 1_500_000u64,
    target_feerate: 0.5f32,
    long_term_feerate: Some(0.3f32),
    min_absolute_fee: 1000u64,
    base_weight: 72u64,
    change_weight: 18u64,
    change_cost: 250u64,
    avg_input_weight: 300u64,
    avg_output_weight: 250u64,
    min_change_value: 1_000u64,
    excess_strategy: ExcessStrategy::ToChange,
};

if let Ok(selection_output) = select_coin(&output_groups, &options) {
    println!("Indexes of the selected utxos = {:?}", selection_output.selected_inputs);
}

```

The `convert_utxo_to_output` logic should be implemented by the user for the respective blockchain protocol.
Note that we can group multiple utxos into a single `OutputGroup`.

Other characteristics of the library:

- Well-documented code, helpful in understanding coin selection theory.
- Minimal possible dependency footprint.
- Minimal possible MSRV (Minimum Supported Rust Version).

## Documentation
For full API documentation, please visit [docs.rs/rust-coinselect](https://docs.rs/rust-coinselect)

## Community

The dev community gathers in a small corner of Discord [here](https://discord.gg/TSSAB3g4Zf) (say hello if you drop there from this readme).

Dev discussions predominantly happen via FOSS (Free and Open-Source Software) best practices, and by using GitHub as the Community Forum.

The Issues, PRs, and Discussions are where all the heavy lifting happens.

Contributions are welcome! Please feel free to submit a [Pull Request](https://github.com/Bitshala-Incubator/rust-coinselect/pulls).