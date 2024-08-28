# Example on using `select_coin` of Coin Selection library

## Overview
The Coin Selection library provides a set of algorithms for coin selection in Bitcoin cryptocurrency transactions systems. The library offers multiple strategies for selecting unspent transaction outputs (UTXOs) to compose a transaction, each with its own advantages and use cases.

## Usage

### **`select_coin`**
Performs optimal coin selection by trying multiple algorithms and selecting the best result. This function runs multiple coin selection algorithms concurrently and chooses the result with the lowest waste metric.

#### Arguments
**_inputs_** - A slice of `OutputGroup` representing available UTXOs.

**_options_** - A `CoinSelectionOpt` struct containing selection parameters and constraints.

#### Returns
Returns a `Result<SelectionOutput, SelectionError>`, where `SelectionOutput` contains the selected inputs and other relevant information.

#### Example
```rust
use crate_name::{select_coin, OutputGroup, CoinSelectionOpt, ExcessStrategy};

let inputs = vec![
    OutputGroup { value: 100000, weight: 100, input_count: 1, is_segwit: true, creation_sequence: Some(3) },
    OutputGroup { value: 200000, weight: 100, input_count: 1, is_segwit: true, creation_sequence: Some(1) },
    OutputGroup { value: 300000, weight: 100, input_count: 1, is_segwit: true, creation_sequence: Some(2) },
];

let options = CoinSelectionOpt {
    target_value: 250000,
    target_feerate: 1.0,
    long_term_feerate: Some(0.5),
    min_absolute_fee: 1000,
    base_weight: 500,
    drain_weight: 50,
    drain_cost: 50,
    cost_per_input: 10,
    cost_per_output: 10,
    min_drain_value: 10000,
    excess_strategy: ExcessStrategy::ToDrain,
};

match select_coin(&inputs, options) {
    Ok(selection) => {
        println!("Selected UTXOs: {:?}", selection.selected_inputs);
        println!("Waste: {}", selection.waste.0);
    },
    Err(e) => println!("Selection failed: {:?}", e),
}
```
```rust
pub fn select_coin(
    inputs: &[OutputGroup],
    options: CoinSelectionOpt,
) -> Result<SelectionOutput, SelectionError> {
    // Implementation details...
}
```

