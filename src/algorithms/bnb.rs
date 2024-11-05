use rand::{rngs::ThreadRng, thread_rng, Rng};

use crate::{
    types::{
        CoinSelectionOpt, MatchParameters, OutputGroup, SelectionError, SelectionOutput,
        WasteMetric,
    },
    utils::{calculate_fee, calculate_waste, effective_value},
};

/// Perform Coinselection via Branch And Bound algorithm.
pub fn select_coin_bnb(
    inputs: &[OutputGroup],
    options: CoinSelectionOpt,
) -> Result<SelectionOutput, SelectionError> {
    let mut selected_inputs: Vec<usize> = vec![];

    // Variable is mutable for decrement of bnb_tries for every iteration of fn bnb
    let mut bnb_tries: u32 = 1_000_000;

    let rng = &mut thread_rng();

    let match_parameters = MatchParameters {
        target_for_match: options.target_value
            + calculate_fee(options.base_weight, options.target_feerate)
            + options.cost_per_output,
        match_range: options.cost_per_input + options.cost_per_output,
        target_feerate: options.target_feerate,
    };

    let mut sorted_inputs: Vec<(usize, OutputGroup)> = inputs
        .iter()
        .enumerate()
        .map(|(index, input)| (index, *input))
        .collect();
    sorted_inputs.sort_by_key(|(_, input)| std::cmp::Reverse(input.value));

    let bnb_selected_coin = bnb(
        &sorted_inputs,
        &mut selected_inputs,
        0,
        0,
        &mut bnb_tries,
        rng,
        &match_parameters,
    );
    match bnb_selected_coin {
        Some(selected_coin) => {
            let accumulated_value: u64 = selected_coin
                .iter()
                .fold(0, |acc, &i| acc + inputs[i].value);
            let accumulated_weight: u32 = selected_coin
                .iter()
                .fold(0, |acc, &i| acc + inputs[i].weight);
            let estimated_fee = 0;
            let waste = calculate_waste(
                &options,
                accumulated_value,
                accumulated_weight,
                estimated_fee,
            );
            let selection_output = SelectionOutput {
                selected_inputs: selected_coin,
                waste: WasteMetric(waste),
            };
            Ok(selection_output)
        }
        None => Err(SelectionError::NoSolutionFound),
    }
}

/// Return empty vec if no solutions are found
///
/// changing the selected_inputs : &[usize] -> &mut Vec<usize>
fn bnb(
    inputs_in_desc_value: &[(usize, OutputGroup)],
    selected_inputs: &mut Vec<usize>,
    acc_eff_value: u64,
    depth: usize,
    bnb_tries: &mut u32,
    rng: &mut ThreadRng,
    match_parameters: &MatchParameters,
) -> Option<Vec<usize>> {
    if acc_eff_value > match_parameters.target_for_match + match_parameters.match_range {
        return None;
    }
    if acc_eff_value >= match_parameters.target_for_match {
        return Some(selected_inputs.to_vec());
    }

    // Capping the number of iterations on the computation
    if *bnb_tries == 0 || depth >= inputs_in_desc_value.len() {
        return None;
    }

    // Decrement of bnb_tries for every iteration
    *bnb_tries -= 1;

    if rng.gen_bool(0.5) {
        // exploring the inclusion branch
        // first include then omit
        let new_effective_value = acc_eff_value
            + effective_value(
                &inputs_in_desc_value[depth].1,
                match_parameters.target_feerate,
            );
        selected_inputs.push(inputs_in_desc_value[depth].0);
        let with_this = bnb(
            inputs_in_desc_value,
            selected_inputs,
            new_effective_value,
            depth + 1,
            bnb_tries,
            rng,
            match_parameters,
        );
        match with_this {
            Some(_) => with_this,
            None => {
                selected_inputs.pop(); // popping out the selected utxo if it does not fit
                bnb(
                    inputs_in_desc_value,
                    selected_inputs,
                    acc_eff_value,
                    depth + 1,
                    bnb_tries,
                    rng,
                    match_parameters,
                )
            }
        }
    } else {
        match bnb(
            inputs_in_desc_value,
            selected_inputs,
            acc_eff_value,
            depth + 1,
            bnb_tries,
            rng,
            match_parameters,
        ) {
            Some(without_this) => Some(without_this),
            None => {
                let new_effective_value = acc_eff_value
                    + effective_value(
                        &inputs_in_desc_value[depth].1,
                        match_parameters.target_feerate,
                    );
                selected_inputs.push(inputs_in_desc_value[depth].0);
                let with_this = bnb(
                    inputs_in_desc_value,
                    selected_inputs,
                    new_effective_value,
                    depth + 1,
                    bnb_tries,
                    rng,
                    match_parameters,
                );
                match with_this {
                    Some(_) => with_this,
                    None => {
                        selected_inputs.pop(); // poping out the selected utxo if it does not fit
                        None
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        algorithms::bnb::select_coin_bnb,
        types::{CoinSelectionOpt, ExcessStrategy, OutputGroup, SelectionError},
    };

    fn setup_basic_output_groups() -> Vec<OutputGroup> {
        vec![
            OutputGroup {
                value: 1000,
                weight: 100,
                input_count: 1,
                creation_sequence: None,
            },
            OutputGroup {
                value: 2000,
                weight: 200,
                input_count: 1,
                creation_sequence: None,
            },
            OutputGroup {
                value: 3000,
                weight: 300,
                input_count: 1,
                creation_sequence: None,
            },
        ]
    }

    fn bnb_setup_options(target_value: u64) -> CoinSelectionOpt {
        CoinSelectionOpt {
            target_value,
            target_feerate: 0.5, // Simplified feerate
            long_term_feerate: None,
            min_absolute_fee: 0,
            base_weight: 10,
            change_weight: 50,
            change_cost: 10,
            cost_per_input: 20,
            cost_per_output: 10,
            min_change_value: 500,
            excess_strategy: ExcessStrategy::ToChange,
        }
    }

    fn test_bnb_solution() {
        // Define the test values
        let values = [
            OutputGroup {
                value: 55000,
                weight: 500,
                input_count: 1,
                creation_sequence: None,
            },
            OutputGroup {
                value: 400,
                weight: 200,
                input_count: 1,
                creation_sequence: None,
            },
            OutputGroup {
                value: 40000,
                weight: 300,
                input_count: 1,
                creation_sequence: None,
            },
            OutputGroup {
                value: 25000,
                weight: 100,
                input_count: 1,
                creation_sequence: None,
            },
            OutputGroup {
                value: 35000,
                weight: 150,
                input_count: 1,
                creation_sequence: None,
            },
            OutputGroup {
                value: 600,
                weight: 250,
                input_count: 1,
                creation_sequence: None,
            },
            OutputGroup {
                value: 30000,
                weight: 120,
                input_count: 1,
                creation_sequence: None,
            },
            OutputGroup {
                value: 5000,
                weight: 50,
                input_count: 1,
                creation_sequence: None,
            },
        ];

        // Adjust the target value to ensure it tests for multiple valid solutions
        let opt = bnb_setup_options(5730);
        let ans = select_coin_bnb(&values, opt);
        if let Ok(selection_output) = ans {
            let expected_solution = vec![7, 5, 1];
            assert_eq!(
                selection_output.selected_inputs, expected_solution,
                "Expected solution {:?}, but got {:?}",
                expected_solution, selection_output.selected_inputs
            );
        } else {
            panic!("Failed to find a solution");
        }
    }

    fn test_bnb_no_solution() {
        let inputs = setup_basic_output_groups();
        let total_input_value: u64 = inputs.iter().map(|input| input.value).sum();
        let impossible_target = total_input_value + 1000;
        let options = bnb_setup_options(impossible_target);
        let result = select_coin_bnb(&inputs, options);
        assert!(
            matches!(result, Err(SelectionError::NoSolutionFound)),
            "Expected NoSolutionFound error, got {:?}",
            result
        );
    }

    #[test]
    fn test_bnb() {
        test_bnb_solution();
        test_bnb_no_solution();
    }
}
