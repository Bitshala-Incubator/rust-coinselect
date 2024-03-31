extern crate rust-coinselect as rcs;

#[cfg(test)]
mod test {
    fn setup_fifo_output_groups() -> Vec<OutputGroup> {
        vec![
            OutputGroup {
                value: 1000,
                weight: 100,
                input_count: 1,
                is_segwit: false,
                creation_sequence: Some(1),
            },
            OutputGroup {
                value: 2000,
                weight: 200,
                input_count: 1,
                is_segwit: false,
                creation_sequence: Some(5000),
            },
            OutputGroup {
                value: 3000,
                weight: 300,
                input_count: 1,
                is_segwit: false,
                creation_sequence: Some(1001),
            },
        ]
    }
    fn setup_options(target_value: u64) -> CoinSelectionOpt {
        CoinSelectionOpt {
            target_value,
            target_feerate: 0.5, // Simplified feerate
            long_term_feerate: None,
            min_absolute_fee: 0,
            base_weight: 10,
            drain_weight: 50,
            drain_cost: 0,
            min_drain_value: 500,
            excess_strategy: ExcessStrategy::ToDrain,
        }
    }
}
#[test]
    fn test_successful_fifo_selection() {
        let inputs = setup_fifo_output_groups();
        let options = setup_options(500); // Seting up target value such that excess exists
        let result = rcs::select_coin_fifo(inputs, options);
        let selection_output = result.unwrap();
        println!("{:?}", selection_output);
        //assert!(!selection_output.selected_inputs.is_empty());
    }
    fn test_fifo() {
        test_successful_fifo_selection();
    }