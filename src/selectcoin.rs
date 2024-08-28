use crate::algorithms::{
    fifo::select_coin_fifo, knapsack::select_coin_knapsack, lowestlarger::select_coin_lowestlarger,
    srd::select_coin_srd,
};
use crate::types::{
    CoinSelectionFn, CoinSelectionOpt, OutputGroup, SelectionError, SelectionOutput, SharedState,
};
use std::sync::{Arc, Mutex};
use std::thread;

pub fn select_coin(
    inputs: &[OutputGroup],
    options: CoinSelectionOpt,
) -> Result<SelectionOutput, SelectionError> {
    let algorithms: Vec<CoinSelectionFn> = vec![
        select_coin_fifo,
        select_coin_lowestlarger,
        select_coin_srd,
        select_coin_knapsack, // Future algorithms can be added here
    ];
    // Shared result for all threads
    let best_result = Arc::new(Mutex::new(SharedState {
        result: Err(SelectionError::NoSolutionFound),
        any_success: false,
    }));
    let mut handles = vec![];
    for &algorithm in &algorithms {
        let best_result_clone = Arc::clone(&best_result);
        let inputs_clone = inputs.to_vec();
        let options_clone = options;
        let handle = thread::spawn(move || {
            let result = algorithm(&inputs_clone, options_clone);
            let mut state = best_result_clone.lock().unwrap();
            match result {
                Ok(selection_output) => {
                    if match &state.result {
                        Ok(current_best) => selection_output.waste.0 < current_best.waste.0,
                        Err(_) => true,
                    } {
                        state.result = Ok(selection_output);
                        state.any_success = true;
                    }
                }
                Err(e) => {
                    if e == SelectionError::InsufficientFunds && !state.any_success {
                        // Only set to InsufficientFunds if no algorithm succeeded
                        state.result = Err(SelectionError::InsufficientFunds);
                    }
                }
            }
        });
        handles.push(handle);
    }
    // Wait for all threads to finish
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
    // Extract the result from the shared state
    Arc::try_unwrap(best_result)
        .expect("Arc unwrap failed")
        .into_inner()
        .expect("Mutex lock failed")
        .result
}

#[cfg(test)]
mod test {

    use crate::{
        selectcoin::select_coin,
        types::{CoinSelectionOpt, ExcessStrategy, OutputGroup, SelectionError},
    };

    fn setup_basic_output_groups() -> Vec<OutputGroup> {
        vec![
            OutputGroup {
                value: 1000,
                weight: 100,
                input_count: 1,
                is_segwit: false,
                creation_sequence: None,
            },
            OutputGroup {
                value: 2000,
                weight: 200,
                input_count: 1,
                is_segwit: false,
                creation_sequence: None,
            },
            OutputGroup {
                value: 3000,
                weight: 300,
                input_count: 1,
                is_segwit: false,
                creation_sequence: None,
            },
        ]
    }

    fn setup_options(target_value: u64) -> CoinSelectionOpt {
        CoinSelectionOpt {
            target_value,
            target_feerate: 0.4, // Simplified feerate
            long_term_feerate: Some(0.4),
            min_absolute_fee: 0,
            base_weight: 10,
            drain_weight: 50,
            drain_cost: 10,
            cost_per_input: 20,
            cost_per_output: 10,
            min_drain_value: 500,
            excess_strategy: ExcessStrategy::ToDrain,
        }
    }

    #[test]
    fn test_select_coin_successful() {
        let inputs = setup_basic_output_groups();
        let options = setup_options(1500);
        let result = select_coin(&inputs, options);
        assert!(result.is_ok());
        let selection_output = result.unwrap();
        assert!(!selection_output.selected_inputs.is_empty());
    }

    #[test]
    fn test_select_coin_insufficient_funds() {
        let inputs = setup_basic_output_groups();
        let options = setup_options(7000); // Set a target value higher than the sum of all inputs
        let result = select_coin(&inputs, options);
        assert!(matches!(result, Err(SelectionError::InsufficientFunds)));
    }
}
