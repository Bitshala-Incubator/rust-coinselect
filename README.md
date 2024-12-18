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
let utxos: Vec<UTXO> = vec![<utxo1>, <utxo2>, ..., <utxon>]; // List of the available UTXOs
let output_groups: Vec<OutputGroup> = utxos.iter().map(|utxo| convert_utxo_to_output(utxo)).collect();
let options = CoinSelectionOpt {
    target_value: 4_000_000u64,
    target_fee_rate: 0.5f32,
    long_term_feerate: Some(0.3f32),
    min_absolute_fee: 1000u64,
    base_weight: 72u64,
    change_weight: 18u64,
    change_cost: 250u64,
    cost_per_input: 300u64,
    cost_per_output: 250u64,
    min_change_value: 1_000u64,
    excess_strategy: ExcessStrategy::ToChange,
};

let selection_output = select_coin(&output_groups, options);
println!("Estimated waste = {}", selection_output.waste);
println!("Indexes of the selected utxos = {}", selection_output.selected_inputs);

let selected_utxos: Vec<UTXO> = selection_output.iter().map(|index| utxos[index]).collect();
```

The `convert_utxo_to_output` logic should be implemented by the user for the respective blockchain protocol.
Note that we can group multiple utxos into a single [`OutputGroup`].

Other characteristics of the library:

- Well-documented code, helpful in understanding coin selection theory.
- Minimal possible dependency footprint.
- Minimal possible MSRV (Minimum Supported Rust Version).

## Community

The dev community gathers in a small corner of Discord [here](https://discord.gg/TSSAB3g4Zf) (say hello if you drop there from this readme).

Dev discussions predominantly happen via FOSS (Free and Open-Source Software) best practices, and by using GitHub as the Community Forum.

The Issues, PRs, and Discussions are where all the heavy lifting happens.
