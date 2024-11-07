use crate::{
    types::{
        CoinSelectionOpt, EffectiveValue, OutputGroup, SelectionError, SelectionOutput,
        WasteMetric, Weight,
    },
    utils::{calculate_accumulated_weight, calculate_fee, calculate_waste, effective_value},
};
use rand::{thread_rng, Rng};
use std::{cmp::Reverse, collections::HashSet};

pub fn select_coin_knapsack(
    inputs: &[OutputGroup],
    options: CoinSelectionOpt,
) -> Result<SelectionOutput, SelectionError> {
    let adjusted_target = options.target_value
        + options.min_change_value
        + calculate_fee(options.base_weight, options.target_feerate);
    let mut smaller_coins = inputs
        .iter()
        .enumerate()
        .filter(|&(_, output_group)| output_group.value < adjusted_target)
        .map(|(index, output_group)| {
            (
                index,
                effective_value(output_group, options.target_feerate),
                output_group.weight,
            )
        })
        .collect::<Vec<_>>();
    smaller_coins.sort_by_key(|&(_, value, _)| Reverse(value));

    knap_sack(adjusted_target, &smaller_coins, options)
}

fn knap_sack(
    adjusted_target: u64,
    smaller_coins: &[(usize, EffectiveValue, Weight)],
    options: CoinSelectionOpt,
) -> Result<SelectionOutput, SelectionError> {
    let mut selected_inputs: HashSet<usize> = HashSet::new();
    let mut accumulated_value: u64 = 0;
    let mut best_set: HashSet<usize> = HashSet::new();
    let mut best_set_value: u64 = u64::MAX;
    let mut rng = thread_rng();
    for _ in 1..=1000 {
        for pass in 1..=2 {
            for &(index, value, _) in smaller_coins {
                let toss_result: bool = rng.gen_bool(0.5);
                if (pass == 2 && !selected_inputs.contains(&index)) || (pass == 1 && toss_result) {
                    selected_inputs.insert(index);
                    accumulated_value += value;
                    if accumulated_value == adjusted_target {
                        let accumulated_weight =
                            calculate_accumulated_weight(smaller_coins, &selected_inputs);
                        let estimated_fees =
                            calculate_fee(accumulated_weight, options.target_feerate);
                        let index_vector: Vec<usize> = selected_inputs.into_iter().collect();
                        let waste: u64 = calculate_waste(
                            &options,
                            accumulated_value,
                            accumulated_weight,
                            estimated_fees,
                        );
                        return Ok(SelectionOutput {
                            selected_inputs: index_vector,
                            waste: WasteMetric(waste),
                        });
                    } else if accumulated_value >= adjusted_target {
                        if accumulated_value < best_set_value {
                            best_set_value = accumulated_value;
                            best_set.clone_from(&selected_inputs);
                        }
                        selected_inputs.remove(&index);
                        accumulated_value -= value;
                    }
                }
            }
        }
        accumulated_value = 0;
        selected_inputs.clear();
    }
    if best_set_value == u64::MAX {
        Err(SelectionError::NoSolutionFound)
    } else {
        let best_set_weight = calculate_accumulated_weight(smaller_coins, &best_set);
        let estimated_fees = calculate_fee(best_set_weight, options.target_feerate);
        let index_vector: Vec<usize> = best_set.into_iter().collect();
        let waste: u64 = calculate_waste(&options, best_set_value, best_set_weight, estimated_fees);
        Ok(SelectionOutput {
            selected_inputs: index_vector,
            waste: WasteMetric(waste),
        })
    }
}

#[cfg(test)]
mod test {

    use crate::{
        algorithms::knapsack::select_coin_knapsack,
        types::{CoinSelectionOpt, ExcessStrategy, OutputGroup, SelectionError},
        utils::calculate_fee,
    };

    const CENT: f64 = 1000000.0;
    const COIN: f64 = 100000000.0;
    const RUN_TESTS: u32 = 100;
    const RUN_TESTS_SLIM: u32 = 10;

    fn knapsack_setup_options(adjusted_target: u64, target_feerate: f32) -> CoinSelectionOpt {
        let min_change_value = 500;
        let base_weight = 10;
        let target_value =
            adjusted_target - min_change_value - calculate_fee(base_weight, target_feerate);
        CoinSelectionOpt {
            target_value,
            target_feerate, // Simplified feerate
            long_term_feerate: Some(0.4),
            min_absolute_fee: 0,
            base_weight,
            change_weight: 50,
            change_cost: 10,
            cost_per_input: 20,
            cost_per_output: 10,
            min_change_value,
            excess_strategy: ExcessStrategy::ToChange,
        }
    }

    fn knapsack_setup_output_groups(
        value: Vec<u64>,
        weights: Vec<u32>,
        target_feerate: f32,
    ) -> Vec<OutputGroup> {
        let mut inputs: Vec<OutputGroup> = Vec::new();
        for (i, j) in value.into_iter().zip(weights.into_iter()) {
            // input value = effective value + fees
            // Example If we want our input to be equal to 1 CENT while being considered by knapsack(effective value), we have to increase the input by the fees to beginwith
            let k = i.saturating_add(calculate_fee(j, target_feerate));
            inputs.push(OutputGroup {
                value: k,
                weight: j,
                input_count: 1,
                creation_sequence: None,
            })
        }
        inputs
    }

    fn knapsack_add_to_output_group(
        inputs: &mut Vec<OutputGroup>,
        value: Vec<u64>,
        weights: Vec<u32>,
        target_feerate: f32,
    ) {
        for (i, j) in value.into_iter().zip(weights.into_iter()) {
            // input value = effective value + fees
            // Example If we want our input to be equal to 1 CENT while being considered by knapsack(effective value), we have to increase the input by the fees to beginwith
            let k = i.saturating_add(calculate_fee(j, target_feerate));
            inputs.push(OutputGroup {
                value: k,
                weight: j,
                input_count: 1,
                creation_sequence: None,
            })
        }
    }

    fn knapsack_test_vectors() {
        let mut inputs_verify: Vec<usize> = Vec::new();
        for _ in 0..RUN_TESTS {
            // Test if Knapsack retruns an Error
            let mut inputs: Vec<OutputGroup> = Vec::new();
            let mut options = knapsack_setup_options(1000, 0.33);
            let mut result = select_coin_knapsack(&inputs, options);
            assert!(matches!(result, Err(SelectionError::NoSolutionFound)));

            // Adding 2 CENT and 1 CENT to the wallet and testing if knapsack can select the two inputs for a 3 CENT Output
            inputs = knapsack_setup_output_groups(
                vec![(2.0 * CENT).round() as u64, (1.0 * CENT).round() as u64],
                vec![130, 100],
                0.56,
            );
            options = knapsack_setup_options((3.0 * CENT).round() as u64, 0.56);
            if let Ok(result) = select_coin_knapsack(&inputs, options) {
                // Checking if knapsack selectes exactly two inputs
                assert_eq!(result.selected_inputs.len(), 2);
                // Checking if the selected inputs are 2 and 1 CENTS
                inputs_verify = vec![0, 1];
                assert!(inputs_verify
                    .iter()
                    .all(|&item| result.selected_inputs.contains(&item)));
            }
            inputs_verify.clear();
            // Adding 20, 10 and 5 CENT to the wallet, totalling 38 CENTS
            knapsack_add_to_output_group(
                &mut inputs,
                vec![
                    (5.0 * CENT).round() as u64,
                    (10.0 * CENT).round() as u64,
                    (20.0 * CENT).round() as u64,
                ],
                vec![100, 10, 50],
                0.56,
            );
            // Testing if knapsack can select 4 inputs (2,5,10,20) CENTS to make 37 CENTS
            options = knapsack_setup_options((37.0 * CENT).round() as u64, 0.56);
            if let Ok(result) = select_coin_knapsack(&inputs, options) {
                // Checking if knapsack selects exactly 4 inputs
                assert_eq!(result.selected_inputs.len(), 4);
                // Checking if the selected inputs are 20, 10, 5, 2 CENTS
                inputs_verify = vec![4, 3, 2, 0];
                assert!(inputs_verify
                    .iter()
                    .all(|&item| result.selected_inputs.contains(&item)));
            }
            inputs_verify.clear();
            // Testing if knapsack can select all the available inputs (2,1,5,10,20) CENTS to make 38 CENTS
            options = knapsack_setup_options((38.0 * CENT).round() as u64, 0.56);
            if let Ok(result) = select_coin_knapsack(&inputs, options) {
                // Cehcking if knapsack selects exactly 5 inputs
                assert_eq!(result.selected_inputs.len(), 5);
                // Cehcking if the selected inputs are 20, 10, 5, 2, 1 CENTS
                inputs_verify = vec![4, 3, 2, 1, 0];
                assert!(inputs_verify
                    .iter()
                    .all(|&item| result.selected_inputs.contains(&item)));
            }
            inputs_verify.clear();
            // Testing if knapsack can select 3 inputs (5,10,20) CENTS to make 34 CENTS
            options = knapsack_setup_options((34.0 * CENT).round() as u64, 0.56);
            if let Ok(result) = select_coin_knapsack(&inputs, options) {
                // Checking if knapsack selects exactly 3 inputs
                assert_eq!(result.selected_inputs.len(), 3);
                // Cehcking if the selected inputs are 20, 10, 5
                inputs_verify = vec![4, 3, 2];
                assert!(inputs_verify
                    .iter()
                    .all(|&item| result.selected_inputs.contains(&item)));
            }
            inputs_verify.clear();
            // Testing if knapsack can select 2 inputs (5,2) CENTS to make 7 CENTS
            options = knapsack_setup_options((7.0 * CENT).round() as u64, 0.56);
            if let Ok(result) = select_coin_knapsack(&inputs, options) {
                // Chekcing if knapsack selects exactly 2 inputs
                assert_eq!(result.selected_inputs.len(), 2);
                // Checking if the selected inputs are 5, 2 CENTS
                inputs_verify = vec![0, 2];
                assert!(inputs_verify
                    .iter()
                    .all(|&item| result.selected_inputs.contains(&item)));
            }
            inputs_verify.clear();
            // Testing if knapsack can select 3 inputs (5,2,1) CENTS to make 8 CENTS
            options = knapsack_setup_options((8.0 * CENT).round() as u64, 0.56);
            if let Ok(result) = select_coin_knapsack(&inputs, options) {
                // Chekcing if knapsack selects exactly 3 inputs
                assert_eq!(result.selected_inputs.len(), 3);
                // Checking if the selected inputs are 5,2,1 CENTS
                inputs_verify = vec![0, 2, 1];
                assert!(inputs_verify
                    .iter()
                    .all(|&item| result.selected_inputs.contains(&item)));
            }
            inputs_verify.clear();
            // Testing if knapsack can select 1 input (10) CENTS to make 9 CENTS
            options = knapsack_setup_options((10.0 * CENT).round() as u64, 0.56);
            if let Ok(result) = select_coin_knapsack(&inputs, options) {
                // Chekcing if knapsack selects exactly 1 inputs
                assert_eq!(result.selected_inputs.len(), 1);
                // Checking if the selected inputs are 10 CENTS
                inputs_verify = vec![10];
                assert!(inputs_verify
                    .iter()
                    .all(|&item| result.selected_inputs.contains(&item)));
            }
            inputs_verify.clear();
            // Clearing the input vector
            inputs.clear();
            // Adding 30, 20, 8, 7,6 CENT to the wallet, totalling 71 CENTS
            // Adding 0.001 CENT to the inputs to account for fees
            inputs = knapsack_setup_output_groups(
                vec![
                    (6.0 * CENT).round() as u64,
                    (7.0 * CENT).round() as u64,
                    (8.0 * CENT).round() as u64,
                    (20.0 * CENT).round() as u64,
                    (30.0 * CENT).round() as u64,
                ],
                vec![100, 200, 100, 10, 5],
                0.77,
            );
            // Testing if Knapsack returns an Error while trying to select inputs totalling 72 CENTS
            options = knapsack_setup_options((72.0 * CENT).round() as u64, 0.77);
            result = select_coin_knapsack(&inputs, options);
            assert!(matches!(result, Err(SelectionError::NoSolutionFound)));
            // Testing if knapsack can select 3 input (6,7,8) CENTS to make 16 CENTS
            options = knapsack_setup_options((16.0 * CENT).round() as u64, 0.77);
            if let Ok(result) = select_coin_knapsack(&inputs, options) {
                // Chekcing if knapsack selects exactly 3 inputs
                assert_eq!(result.selected_inputs.len(), 3);
                // Checking if the selected inputs are 6,7,8 CENTS
                inputs_verify = vec![0, 1, 2];
                assert!(inputs_verify
                    .iter()
                    .all(|&item| result.selected_inputs.contains(&item)));
            }
            inputs_verify.clear();
            // Adding 5 CENT to the wallet, totalling 76 CENTS
            knapsack_add_to_output_group(
                &mut inputs,
                vec![(5.0 * CENT).round() as u64],
                vec![10],
                0.77,
            );
            // Testing if knapsack can select 3 input (5,6,7) CENTS to make 16 CENTS
            options = knapsack_setup_options((16.0 * CENT).round() as u64, 0.77);
            if let Ok(result) = select_coin_knapsack(&inputs, options) {
                // Chekcing if knapsack selects exactly 3 inputs
                assert_eq!(result.selected_inputs.len(), 3);
                // Checking if the selected inputs are 6,7,8 CENTS
                inputs_verify = vec![0, 1, 5];
                assert!(inputs_verify
                    .iter()
                    .all(|&item| result.selected_inputs.contains(&item)));
            }
            inputs_verify.clear();

            // Adding 18 CENT to the wallet, totalling 94 CENTS
            knapsack_add_to_output_group(
                &mut inputs,
                vec![(18.0 * CENT).round() as u64],
                vec![1],
                0.77,
            );
            // Testing if knapsack can select 2 input (5,6) CENTS to make 11 CENTS
            options = knapsack_setup_options((11.0 * CENT).round() as u64, 0.77);
            if let Ok(result) = select_coin_knapsack(&inputs, options) {
                // Chekcing if knapsack selects exactly 2 inputs
                assert_eq!(result.selected_inputs.len(), 2);
                // Checking if the selected input is 5,6 CENTS
                inputs_verify = vec![0, 5];
                assert!(inputs_verify
                    .iter()
                    .all(|&item| result.selected_inputs.contains(&item)));
            }
            inputs_verify.clear();
            // Clearing the input vector
            inputs.clear();
            // Adding 0.1, 0.2, 0.3, 0.4, 0.5 CENT to the wallet, totalling 1.5 CENTS
            inputs = knapsack_setup_output_groups(
                vec![
                    (0.101 * CENT).round() as u64,
                    (0.201 * CENT).round() as u64,
                    (0.301 * CENT).round() as u64,
                    (0.401 * CENT).round() as u64,
                    (0.501 * CENT).round() as u64,
                ],
                vec![14, 45, 6, 10, 100],
                0.56,
            );
            // Testing if knapsack can select 3 input (0.1, 0.4, 0.5| 0.2, 0.3, 0.5) CENTS to make 1 CENTS
            options = knapsack_setup_options((1.0 * CENT).round() as u64, 0.56);
            if let Ok(result) = select_coin_knapsack(&inputs, options) {
                // Chekcing if knapsack selects exactly 3 inputs
                assert_eq!(result.selected_inputs.len(), 3);
                // Checking if the selected input is 0.1,0.4,0.5 CENTS
                inputs_verify = vec![0, 3, 4];
                let valid_inputs_1 = inputs_verify
                    .iter()
                    .all(|&item| result.selected_inputs.contains(&item));
                inputs_verify.clear();
                inputs_verify = vec![1, 2, 4];
                let valid_inputs_2 = inputs_verify
                    .iter()
                    .all(|&item| result.selected_inputs.contains(&item));
                assert!(valid_inputs_1 || valid_inputs_2);
            }
            inputs_verify.clear();
            // Mt.Gox Test
            inputs.clear();
            // Adding 11, 50,000 COINS to the input
            inputs = knapsack_setup_output_groups(
                vec![
                    (50000.0 * COIN).round() as u64,
                    (50000.0 * COIN).round() as u64,
                    (50000.0 * COIN).round() as u64,
                    (50000.0 * COIN).round() as u64,
                    (50000.0 * COIN).round() as u64,
                    (50000.0 * COIN).round() as u64,
                    (50000.0 * COIN).round() as u64,
                    (50000.0 * COIN).round() as u64,
                    (50000.0 * COIN).round() as u64,
                    (50000.0 * COIN).round() as u64,
                    (50000.0 * COIN).round() as u64,
                ],
                vec![1, 20, 3, 200, 150, 5, 88, 93, 101, 34, 17],
                0.59,
            );
            // Testing if knapsack can select 10 inputs to make 500,000 COINS
            options = knapsack_setup_options((500000.0 * COIN).round() as u64, 0.59);
            if let Ok(result) = select_coin_knapsack(&inputs, options) {
                // Chekcing if knapsack selects exactly 10 inputs
                assert_eq!(result.selected_inputs.len(), 10);
            }
            // Clearing the input vectors
            inputs.clear();
            // Adding 0.4, 0.6, 0.8, 1111 CENTS to the wallet totalling 1112.8 CENTS
            inputs = knapsack_setup_output_groups(
                vec![
                    (0.4 * CENT).round() as u64,
                    (0.6 * CENT).round() as u64,
                    (0.8 * CENT).round() as u64,
                    (1111.0 * CENT).round() as u64,
                ],
                vec![14, 45, 6, 10],
                0.56,
            );
            // Testing if knapsack can select 2 input (0.4,0.6) CENTS to make 1 CENTs
            options = knapsack_setup_options((1.0 * CENT).round() as u64, 0.56);
            if let Ok(result) = select_coin_knapsack(&inputs, options) {
                // Chekcing if knapsack selects exactly 2 inputs
                assert_eq!(result.selected_inputs.len(), 2);
                // Checking if the selected input is 0.4,0.6 CENTS
                inputs_verify = vec![0, 1];
                assert!(inputs_verify
                    .iter()
                    .all(|&item| result.selected_inputs.contains(&item)));
            }
            inputs_verify.clear();
            // Clearing the input vectors
            inputs.clear();
            // Adding 0.05, 1, 100 CENTS to the wallet totalling 101.05 CENTS
            inputs = knapsack_setup_output_groups(
                vec![
                    (100.0 * CENT).round() as u64,
                    (1.0 * CENT).round() as u64,
                    (0.05 * CENT).round() as u64,
                ],
                vec![14, 45, 6],
                0.56,
            );
            // Testing if knapsack can select 2 input (100,1) CENTS to make 100.01 CENTs, therby avoiding creating small change if 100 & 0.05 is chosen
            options = CoinSelectionOpt {
                target_value: (100.01 * CENT).round() as u64,
                target_feerate: 0.56, // Simplified feerate
                long_term_feerate: Some(0.4),
                min_absolute_fee: 0,
                base_weight: 10,
                change_weight: 50,
                change_cost: 10,
                cost_per_input: 20,
                cost_per_output: 10,
                min_change_value: (0.05 * CENT).round() as u64, // Setting minimum change value = 0.05 CENT. This will make the algorithm to avoid creating small change.
                excess_strategy: ExcessStrategy::ToChange,
            };
            if let Ok(result) = select_coin_knapsack(&inputs, options) {
                // Chekcing if knapsack selects exactly 2 inputs
                assert_eq!(result.selected_inputs.len(), 2);
                // Checking if the selected input is 0.4,0.6 CENTS
                inputs_verify = vec![0, 1];
                assert!(inputs_verify
                    .iter()
                    .all(|&item| result.selected_inputs.contains(&item)));
            }
            inputs_verify.clear();
            // Clearing the input vectors
            inputs.clear();
        }
        // Test with multiple inputs
        let mut inputs: Vec<OutputGroup> = Vec::new();
        let mut amt = 1500;
        // Increase the input amoutn startig from 1500 Sats to COIN = 100000000 Sats in multiples of 10
        while amt < COIN as u64 {
            inputs.clear();
            // Declare value and weights vectors
            let mut input_value: Vec<u64> = Vec::new();
            let mut input_weight: Vec<u32> = Vec::new();
            for _ in 0..676 {
                // Populate the vectors with the same value 'amt' and weight = 23 for 676 times
                // Using 676 as (old MAX_STANDARD_TX_SIZE = 100000)/(148 bytes per input) = 676
                input_value.push(amt);
                input_weight.push(23);
            }
            let inputs = knapsack_setup_output_groups(input_value, input_weight, 0.34);
            // Setting the selection target to 2000 sats
            let options = knapsack_setup_options(2000, 0.34);
            // performing the assertion operation 10 times
            for _ in 0..RUN_TESTS_SLIM {
                if let Ok(result) = select_coin_knapsack(&inputs, options) {
                    if let Some(amt_in_inputs) = inputs.first() {
                        // Checking if the (input's value) - 2000 is less than CENT
                        // If so, more than one input is required to meet the selection target of 2000 sats
                        if amt_in_inputs.value.checked_sub(2000) < Some(CENT as u64) {
                            // calculating the no.of inputs that will be required to meet the selection target of 2000 sats
                            let return_size = ((2000.0) / amt as f64).ceil();
                            assert_eq!(result.selected_inputs.len(), return_size as usize);
                        } else {
                            // If (input's value) - 2000 is greater than CENT, then only one input is required to meet the selection target of 2000 sats
                            assert_eq!(result.selected_inputs.len(), 1);
                        }
                    } else {
                        println!("unable to access 0th element of input vector");
                    }
                }
            }
            amt *= 10;
        }
        inputs.clear();
        // Testing for Randomness
        // Declare input value and weights vectors
        let mut input_value: Vec<u64> = Vec::new();
        let mut input_weight: Vec<u32> = Vec::new();
        for _ in 0..=100 {
            // Populate the vectors with the same value, COIN = 100000000 sats, and weight = 23 for 100 times (to create 100 identical inputs)
            input_value.push(COIN as u64);
            input_weight.push(23);
        }
        // Setting up inputs
        let mut inputs = knapsack_setup_output_groups(input_value, input_weight, 0.34);
        // Setting the selection target to 50*COIN sats
        let options = knapsack_setup_options((50.0 * COIN).round() as u64, 0.34);
        let mut selected_input_1: Vec<usize> = Vec::new();
        let mut selected_input_2: Vec<usize> = Vec::new();
        for _ in 0..RUN_TESTS {
            if let Ok(result) = select_coin_knapsack(&inputs, options) {
                selected_input_1.clone_from(&result.selected_inputs);
            }
            if let Ok(result) = select_coin_knapsack(&inputs, options) {
                selected_input_2.clone_from(&result.selected_inputs);
            }
            // Checking if the selected inputs, in two consequtive calls of the knapsack function are not the same
            assert_ne!(selected_input_1, selected_input_2);
        }
        selected_input_1.clear();
        selected_input_2.clear();
        // Adding 5, 10, 15, 20, 25 CENT to the wallet, Totalling 175,000,000 SATS
        knapsack_add_to_output_group(
            &mut inputs,
            vec![
                (5.0 * CENT).round() as u64,
                (10.0 * CENT).round() as u64,
                (15.0 * CENT).round() as u64,
                (20.0 * CENT).round() as u64,
                (25.0 * CENT).round() as u64,
            ],
            vec![100, 10, 50, 52, 13],
            0.34,
        );
    }

    #[test]
    fn test_knapsack() {
        knapsack_test_vectors();
    }
}
