#![allow(unused)]

//! A blockchain-agnostic Rust Coinselection library


use rand::{rngs::ThreadRng, seq::SliceRandom, Rng};
use std::{option, vec};

/// A [`OutputGroup`] represents an input candidate for Coinselection. This can either be a
/// single UTXO, or a group of UTXOs that should be spent together.
/// The library user is responsible for crafting this structure correctly. Incorrect representation of this
/// structure will cause incorrect selection result.
#[derive(Debug, Clone, Copy)]
pub struct OutputGroup {
    /// Total value of the UTXO(s) that this [`WeightedValue`] represents.
    pub value: u64,
    /// Total weight of including this/these UTXO(s).
    /// `txin` fields: `prevout`, `nSequence`, `scriptSigLen`, `scriptSig`, `scriptWitnessLen`,
    /// `scriptWitness` should all be included.
    pub weight: u32,
    /// The total number of inputs; so we can calculate extra `varint` weight due to `vin` length changes.
    pub input_count: usize,
    /// Whether this [`OutputGroup`] contains at least one segwit spend.
    pub is_segwit: bool,
    /// Relative Creation sequence for this group. Only used for FIFO selection. Specify None, if FIFO
    /// selection is not required.
    /// Sequqence numbers are arbitrary index only to denote relative age of utxo group among a set of groups.
    /// To denote the oldest utxo group, give them a sequence number of Some(0).
    pub creation_sequqence: Option<u32>,
}

/// A set of Options that guides the CoinSelection algorithms. These are inputs specified by the
/// user to perform coinselection to achieve a set a target parameters.
#[derive(Debug, Clone, Copy)]
pub struct CoinSelectionOpt {
    /// The value we need to select.
    pub target_value: u64,

    /// The feerate we should try and achieve in sats per weight unit.
    pub target_feerate: f32,
    /// The feerate
    pub long_term_feerate: Option<f32>, // TODO: Maybe out of scope? (waste)
    /// The minimum absolute fee. I.e., needed for RBF.
    pub min_absolute_fee: u64,

    /// The weight of the template transaction, including fixed fields and outputs.
    pub base_weight: u32,
    /// Additional weight if we include the drain (change) output.
    pub drain_weight: u32,
    /// Weight of spending the drain (change) output in the future.
    pub spend_drain_weight: u32, // TODO: Maybe out of scope? (waste)

    /// Minimum value allowed for a drain (change) output.
    pub min_drain_value: u64,
}

/// Strategy to decide what to do with the excess amount.
#[derive(Clone, Copy, Debug)]
pub enum ExcessStrategy {
    ToFee,
    ToRecipient,
    ToDrain,
}

/// Error Describing failure of a selection attempt, on any subset of inputs
#[derive(Debug)]
pub enum SelectionError {
    InsufficientFunds,
    NoSolutionFound,
}

/// Calculated waste for a specific selection.
/// This is used to compare various selection algorithm and find the most
/// optimizewd solution, represented by least [WasteMetric] value.
#[derive(Debug)]
pub struct WasteMetric(u64);

/// The result of selection algorithm
pub struct SelectionOutput {
    /// The selected inputs
    pub selected_inputs: Vec<u32>,
    /// The waste amount, for the above inputs
    pub waste: WasteMetric,
}

/// Wrapped in a struct or else input for fn bnb takes too many arguments - 9/7, 
/// Leading to usage of stack instead of registers - https://users.rust-lang.org/t/avoiding-too-many-arguments-passing-to-a-function/103581
/// Fit in : 1 XMM register, 1 GPR
#[derive(Debug)]
pub struct MatchParameters {
    target_for_match: u64,
    match_range: u64,
    target_feerate: f32,
}

/// Perform Coinselection via Branch And Bound algorithm.
pub fn select_coin_bnb(
    inputs: &[OutputGroup],
    options: CoinSelectionOpt,
    rng: &mut ThreadRng,
) -> Result<SelectionOutput, SelectionError> {
    let mut selected_inputs: Vec<usize> = vec![];

    /// Variable is mutable for decrement of bnb_tries for every iteration of fn bnb
    let mut bnb_tries: u32 = 1_000_000;

    let match_parameters = MatchParameters {
        target_for_match: options.target_value + calculate_fee(options.base_weight, options.target_feerate) + options.cost_per_output,
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
                inputs,
                &selected_inputs,
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
// changing the selected_inputs : &[usize] -> &mut Vec<usize>
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

    // Decrement of bnb_tries for every iteration
    *bnb_tries -= 1;
    /// Capping the number of iterations on the computation
    if *bnb_tries == 0 || depth >= inputs_in_desc_value.len() {
        return None;
    }
    if rng.gen_bool(0.5) {
        // exploring the inclusion branch
        // first include then omit
        let new_effective_value =
            acc_eff_value + effective_value(&inputs_in_desc_value[depth].1, match_parameters.target_feerate);
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
        /// Avoided creation of intermediate variable, for marginally better performance.
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
                    + effective_value(&inputs_in_desc_value[depth].1, match_parameters.target_feerate);
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
                        None // this may or may not be correct
                    }
                }
            }
        }
    }
}

/// Perform Coinselection via Knapsack solver.
pub fn select_coin_knapsack(
    inputs: Vec<OutputGroup>,
    opitons: CoinSelectionOpt,
    excess_strategy: ExcessStrategy,
) -> Result<SelectionOutput, SelectionError> {
    unimplemented!()
}

/// Perform Coinselection via Lowest Larger algorithm.
/// Return NoSolutionFound, if no solution exists.
pub fn select_coin_lowestlarger(
    inputs: Vec<OutputGroup>,
    opitons: CoinSelectionOpt,
    excess_strategy: ExcessStrategy,
) -> Result<SelectionOutput, SelectionError> {
    unimplemented!()
}

/// Perform Coinselection via First-In-First-Out algorithm.
/// Return NoSolutionFound, if no solution exists.
pub fn select_coin_fifo(
    inputs: Vec<OutputGroup>,
    opitons: CoinSelectionOpt,
    excess_strategy: ExcessStrategy,
) -> Result<SelectionOutput, SelectionError> {
    unimplemented!()
}

/// Perform Coinselection via Single Random Draw.
/// Return NoSolutionFound, if no solution exists.
pub fn select_coin_srd(
    inputs: &[OutputGroup],
    options: CoinSelectionOpt,
) -> Result<SelectionOutput, SelectionError> {
    // Randomize the inputs order to simulate the random draw
    let mut rng = thread_rng();

    // In out put we need to specify the indexes of the inputs in the given order
    // So keep track of the indexes when randomiz ing the vec
    let mut randomized_inputs: Vec<_> = inputs.iter().enumerate().collect();

    // Randomize the inputs order to simulate the random draw
    let mut rng = thread_rng();
    randomized_inputs.shuffle(&mut rng);

    let mut accumulated_value = 0;
    let mut selected_inputs = Vec::new();
    let mut accumulated_weight = 0;
    let mut estimated_fee = 0;
    let mut input_counts = 0;

    let necessary_target = options.target_value
        + options.min_drain_value
        + calculate_fee(options.base_weight, options.target_feerate);

    for (index, input) in randomized_inputs {
        selected_inputs.push(index);
        accumulated_value += input.value;
        accumulated_weight += input.weight;
        input_counts += input.input_count;

        estimated_fee = calculate_fee(accumulated_weight, options.target_feerate);

        if accumulated_value
            >= options.target_value
                + options.min_drain_value
                + estimated_fee.max(options.min_absolute_fee)
        {
            break;
        }
    }

    if accumulated_value
        < options.target_value
            + options.min_drain_value
            + estimated_fee.max(options.min_absolute_fee)
    {
        return Err(SelectionError::InsufficientFunds);
    }
    // accumulated_weight += weightof(input_counts)?? TODO
    let waste = calculate_waste(
        inputs,
        &selected_inputs,
        &options,
        accumulated_value,
        accumulated_weight,
        estimated_fee,
    );

    Ok(SelectionOutput {
        selected_inputs,
        waste: WasteMetric(waste),
    })
}
/// The Global Coinselection API that performs all the algorithms and proudeces result with least [WasteMetric].
/// At least one selection solution should be found.
pub fn select_coin(
    inputs: Vec<OutputGroup>,
    opitons: CoinSelectionOpt,
    excess_strategy: ExcessStrategy,
) -> Result<SelectionOutput, SelectionError> {
    unimplemented!()
}

#[inline]
fn calculate_waste(
    inputs: &[OutputGroup],
    selected_inputs: &[usize],
    options: &CoinSelectionOpt,
    accumulated_value: u64,
    accumulated_weight: u32,
    estimated_fee: u64,
) -> u64 {
    let mut waste: u64 = 0;

    if let Some(long_term_feerate) = options.long_term_feerate {
        waste += (estimated_fee as f32
            - selected_inputs.len() as f32 * long_term_feerate * accumulated_weight as f32)
            .ceil() as u64;
    }

    if options.excess_strategy != ExcessStrategy::ToDrain {
        waste += accumulated_value - options.target_value - estimated_fee;
    } else {
        waste += options.drain_cost;
    }

    waste
}

#[inline]
fn calculate_fee(weight: u32, rate: f32) -> u64 {
    (weight as f32 * rate).ceil() as u64
}

/// Returns the effective value which is the actual value minus the estimated fee of the OutputGroup
#[inline]
fn effective_value(output: &OutputGroup, feerate: f32) -> u64 {
    output
        .value
        .saturating_sub(calculate_fee(output.weight, feerate))
}

#[cfg(test)]
mod test {

    use super::*;

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
    fn setup_output_groups_withsequence() -> Vec<OutputGroup> {
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
    fn setup_lowestlarger_output_groups() -> Vec<OutputGroup> {
        vec![
            OutputGroup {
                value: 100,
                weight: 100,
                input_count: 1,
                is_segwit: false,
                creation_sequence: None,
            },
            OutputGroup {
                value: 1500,
                weight: 200,
                input_count: 1,
                is_segwit: false,
                creation_sequence: None,
            },
            OutputGroup {
                value: 3400,
                weight: 300,
                input_count: 1,
                is_segwit: false,
                creation_sequence: None,
            },
            OutputGroup {
                value: 2200,
                weight: 150,
                input_count: 1,
                is_segwit: false,
                creation_sequence: None,
            },
            OutputGroup {
                value: 1190,
                weight: 200,
                input_count: 1,
                is_segwit: false,
                creation_sequence: None,
            },
            OutputGroup {
                value: 3300,
                weight: 100,
                input_count: 1,
                is_segwit: false,
                creation_sequence: None,
            },
            OutputGroup {
                value: 1000,
                weight: 190,
                input_count: 1,
                is_segwit: false,
                creation_sequence: None,
            },
            OutputGroup {
                value: 2000,
                weight: 210,
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
            OutputGroup {
                value: 2250,
                weight: 250,
                input_count: 1,
                is_segwit: false,
                creation_sequence: None,
            },
            OutputGroup {
                value: 190,
                weight: 220,
                input_count: 1,
                is_segwit: false,
                creation_sequence: None,
            },
            OutputGroup {
                value: 1750,
                weight: 170,
                input_count: 1,
                is_segwit: false,
                creation_sequence: None,
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
            drain_cost: 10,
            cost_per_input: 20,
            cost_per_output: 10,
            min_drain_value: 500,
            excess_strategy: ExcessStrategy::ToDrain,
        }
    }
    fn knapsack_setup_options(adjusted_target: u64, target_feerate: f32) -> CoinSelectionOpt {
        let min_drain_value = 500;
        let base_weight = 10;
        let target_value =
            adjusted_target - min_drain_value - calculate_fee(base_weight, target_feerate);
        CoinSelectionOpt {
            target_value,
            target_feerate, // Simplified feerate
            long_term_feerate: Some(0.4),
            min_absolute_fee: 0,
            base_weight,
            drain_weight: 50,
            drain_cost: 10,
            cost_per_input: 20,
            cost_per_output: 10,
            min_drain_value,
            excess_strategy: ExcessStrategy::ToDrain,
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
                is_segwit: false,
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
                is_segwit: false,
                creation_sequence: None,
            })
        }
    }
    
    fn bnb_setup_options(target_value: u64) -> CoinSelectionOpt {
        CoinSelectionOpt {
            target_value,
            target_feerate: 0.5, // Simplified feerate
            long_term_feerate: None,
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

    fn test_bnb_solution() {
        // Define the test values
        let values = [
            OutputGroup {
                value: 55000,
                weight: 500,
                input_count: 1,
                is_segwit: false,
                creation_sequence: None,
            },
            OutputGroup {
                value: 400,
                weight: 200,
                input_count: 1,
                is_segwit: false,
                creation_sequence: None,
            },
            OutputGroup {
                value: 40000,
                weight: 300,
                input_count: 1,
                is_segwit: false,
                creation_sequence: None,
            },
            OutputGroup {
                value: 25000,
                weight: 100,
                input_count: 1,
                is_segwit: false,
                creation_sequence: None,
            },
            OutputGroup {
                value: 35000,
                weight: 150,
                input_count: 1,
                is_segwit: false,
                creation_sequence: None,
            },
            OutputGroup {
                value: 600,
                weight: 250,
                input_count: 1,
                is_segwit: false,
                creation_sequence: None,
            },
            OutputGroup {
                value: 30000,
                weight: 120,
                input_count: 1,
                is_segwit: false,
                creation_sequence: None,
            },
            OutputGroup {
                value: 5000,
                weight: 50,
                input_count: 1,
                is_segwit: false,
                creation_sequence: None,
            },
        ];

        // Adjust the target value to ensure it tests for multiple valid solutions
        let opt = bnb_setup_options(5730);
        let ans = select_coin_bnb(&values, opt, &mut rand::thread_rng());
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
        let result = select_coin_bnb(&inputs, options, &mut rand::thread_rng());
        assert!(
            matches!(result, Err(SelectionError::NoSolutionFound)),
            "Expected NoSolutionFound error, got {:?}",
            result
        );
    }

    fn test_successful_selection() {
        let mut inputs = setup_basic_output_groups();
        let mut options = setup_options(2500);
        let mut result = select_coin_srd(&inputs, options);
        assert!(result.is_ok());
        let mut selection_output = result.unwrap();
        assert!(!selection_output.selected_inputs.is_empty());

        inputs = setup_output_groups_withsequence();
        options = setup_options(500);
        result = select_coin_fifo(&inputs, options);
        assert!(result.is_ok());
        selection_output = result.unwrap();
        assert!(!selection_output.selected_inputs.is_empty());
    }

    fn test_insufficient_funds() {
        let inputs = setup_basic_output_groups();
        let options = setup_options(7000); // Set a target value higher than the sum of all inputs
        let result = select_coin_srd(&inputs, options);
        assert!(matches!(result, Err(SelectionError::InsufficientFunds)));
    }
    #[test]
    fn test_srd() {
        // Perform SRD selection of set of test values.
    }

    #[test]
    fn test_knapsack() {
        // Perform Knapsack selection of set of test values.
    }

    #[test]
    fn test_fifo() {
        // Perform FIFO selection of set of test values.
    }

    #[test]
    fn test_bnb() {
        test_bnb_solution();
        test_bnb_no_solution();
    }

    #[test]
    fn test_lowestlarger_successful() {
        let mut inputs = setup_lowestlarger_output_groups();
        let mut options = setup_options(20000);
        let result = select_coin_lowestlarger(&inputs, options);
        assert!(result.is_ok());
        let selection_output = result.unwrap();
        assert!(!selection_output.selected_inputs.is_empty());
    }

    #[test]
    fn test_lowestlarger_insufficient() {
        let mut inputs = setup_lowestlarger_output_groups();
        let mut options = setup_options(40000);
        let result = select_coin_lowestlarger(&inputs, options);
        assert!(matches!(result, Err(SelectionError::InsufficientFunds)));
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