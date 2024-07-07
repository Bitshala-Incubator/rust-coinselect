#![allow(unused)]

//! A blockchain-agnostic Rust Coinselection library

use rand::{seq::SliceRandom, thread_rng, Rng};
use std::cmp::Reverse;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

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
    /// sequence numbers are arbitrary index only to denote relative age of utxo group among a set of groups.
    /// To denote the oldest utxo group, give them a sequence number of Some(0).
    pub creation_sequence: Option<u32>,
}

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
    pub drain_cost: u64,

    /// Estimate of cost of spending an input
    pub cost_per_input: u64,

    /// Estimate of cost of spending the output
    pub cost_per_output: u64,

    /// Minimum value allowed for a drain (change) output.
    pub min_drain_value: u64,

    /// Strategy to use the excess value other than fee and target
    pub excess_strategy: ExcessStrategy,
}

/// Strategy to decide what to do with the excess amount.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// Wastemetric, of a selection of inputs, is measured in satoshis. It helps evaluate the selection made by different algorithms in the context of the current and long term fee rate.
/// It is used to strike a balance between wanting to minimize the current transaction's fees versus minimizing the overall fees paid by the wallet during its lifetime.
/// During high fee rate environment, selecting fewer number of inputs will help minimize the transaction fees.
/// During low fee rate environment, slecting more number of inputs will help minimize the over all fees paid by the wallet during its lifetime.
/// This is used to compare various selection algorithm and find the most
/// optimizewd solution, represented by least [WasteMetric] value.
#[derive(Debug)]
pub struct WasteMetric(u64);

/// The result of selection algorithm
#[derive(Debug)]
pub struct SelectionOutput {
    /// The selected input indices, refers to the indices of the inputs Slice Reference
    pub selected_inputs: Vec<usize>,
    /// The waste amount, for the above inputs
    pub waste: WasteMetric,
}

/// Perform Coinselection via Branch And Bound algorithm.
pub fn select_coin_bnb(
    inputs: &[OutputGroup],
    options: CoinSelectionOpt,
) -> Result<SelectionOutput, SelectionError> {
    unimplemented!()
}

/// Return empty vec if no solutions are found
fn bnb(
    inputs_in_desc_value: &[(usize, OutputGroup)],
    selected_inputs: &[usize],
    effective_value: u64,
    depth: usize,
    bnp_tries: u32,
    options: &CoinSelectionOpt,
) -> Vec<usize> {
    unimplemented!()
}

/// Perform Coinselection via Knapsack solver.
pub fn select_coin_knapsack(
    inputs: &[OutputGroup],
    options: CoinSelectionOpt,
) -> Result<SelectionOutput, SelectionError> {
    let mut adjusted_target = options.target_value
        + options.min_drain_value
        + calculate_fee(options.base_weight, options.target_feerate);
    let mut smaller_coins = inputs
        .iter()
        .enumerate()
        .filter(|&(index, output_group)| output_group.value < adjusted_target)
        .map(|(index, output_group)| (index, *output_group))
        .collect::<Vec<_>>();
    // Sorting smaller_coins in descending order
    smaller_coins.sort_by_key(|&(_, output_group)| Reverse(output_group.value));

    knap_sack(adjusted_target, &smaller_coins, inputs, options)
}

/// adjusted_target should be target value plus estimated fee
/// smaller_coins is a slice of pair where the usize refers to the index of the OutputGroup in the inputs given
/// smaller_coins should be sorted in descending order based on the value of the OutputGroup, and every OutputGroup value should be less than adjusted_target
fn calculate_accumulated_weight(
    inputs: &[(usize, OutputGroup)],
    selected_inputs: &HashSet<usize>,
) -> u32 {
    let mut accumulated_weight: u32 = 0;
    for &(index, u) in inputs {
        if selected_inputs.contains(&index) {
            accumulated_weight += u.weight;
        }
    }
    accumulated_weight
}
fn knap_sack(
    adjusted_target: u64,
    smaller_coins: &[(usize, OutputGroup)],
    inputs: &[OutputGroup],
    options: CoinSelectionOpt,
) -> Result<SelectionOutput, SelectionError> {
    let mut selected_inputs: HashSet<(usize)> = HashSet::new();
    let mut accumulated_value: u64 = 0;
    let mut best_set: HashSet<(usize)> = HashSet::new();
    // Assigning infinity to beginwith
    let mut best_set_value: u64 = u64::MAX;
    let mut rng = thread_rng();
    for i in 1..=1000 {
        for pass in 1..=2 {
            for &(index, coin) in smaller_coins {
                //Simulate a coin toss
                let toss_result: bool = rng.gen_bool(0.5);
                if (pass == 2 && !selected_inputs.contains(&index)) || (pass == 1 && toss_result) {
                    // Including the UTXO in the selected inputs
                    selected_inputs.insert(index);
                    // Adding the effective value of an input (value - estimated fee)
                    accumulated_value += effective_value(&coin, options.target_feerate);
                    if accumulated_value == adjusted_target {
                        // Perfect Match, Return the HashSet selected_inputs
                        // Calculating the weight of elements in the selected_inputs hashset
                        let accumulated_weight =
                            calculate_accumulated_weight(smaller_coins, &selected_inputs);
                        let estimated_fees =
                            calculate_fee(accumulated_weight, options.target_feerate);
                        let index_vector: Vec<usize> = selected_inputs.into_iter().collect();
                        let waste: u64 = calculate_waste(
                            inputs,
                            &index_vector,
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
                        if (accumulated_value < best_set_value) {
                            // New best_set found
                            best_set_value = accumulated_value;
                            best_set.clone_from(&selected_inputs);
                        }
                        // Removing the last UTXO that raised selection_sum above adjusted_target to try to find a smaller set
                        selected_inputs.remove(&index);
                        accumulated_value -= coin.value;
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
        // Calculating the weight of elements in the selected inputs
        let best_set_weight = calculate_accumulated_weight(smaller_coins, &best_set);
        // Calculating the estimated fees for the selected inputs
        let estimated_fees = calculate_fee(best_set_weight, options.target_feerate);
        let index_vector: Vec<usize> = best_set.into_iter().collect();
        // Calculating waste
        let waste: u64 = calculate_waste(
            inputs,
            &index_vector,
            &options,
            best_set_value,
            best_set_weight,
            estimated_fees,
        );
        Ok(SelectionOutput {
            selected_inputs: index_vector,
            waste: WasteMetric(waste),
        })
    }
}

/// Perform Coinselection via Lowest Larger algorithm.
/// Return NoSolutionFound, if no solution exists.
pub fn select_coin_lowestlarger(
    inputs: &[OutputGroup],
    options: CoinSelectionOpt,
) -> Result<SelectionOutput, SelectionError> {
    let mut accumulated_value: u64 = 0;
    let mut accumulated_weight: u32 = 0;
    let mut selected_inputs: Vec<usize> = Vec::new();
    let mut estimated_fees: u64 = 0;
    let target = options.target_value + options.min_drain_value;

    let mut sorted_inputs: Vec<_> = inputs.iter().enumerate().collect();
    sorted_inputs.sort_by_key(|(_, input)| effective_value(input, options.target_feerate));

    let mut index = sorted_inputs.partition_point(|(_, input)| {
        input.value <= (target + calculate_fee(input.weight, options.target_feerate))
    });

    for (idx, input) in sorted_inputs.iter().take(index).rev() {
        accumulated_value += input.value;
        accumulated_weight += input.weight;
        estimated_fees = calculate_fee(accumulated_weight, options.target_feerate);
        selected_inputs.push(*idx);

        if accumulated_value >= (target + estimated_fees.max(options.min_absolute_fee)) {
            break;
        }
    }

    if accumulated_value < (target + estimated_fees.max(options.min_absolute_fee)) {
        for (idx, input) in sorted_inputs.iter().skip(index) {
            accumulated_value += input.value;
            accumulated_weight += input.weight;
            estimated_fees = calculate_fee(accumulated_weight, options.target_feerate);
            selected_inputs.push(*idx);

            if accumulated_value >= (target + estimated_fees.max(options.min_absolute_fee)) {
                break;
            }
        }
    }

    if accumulated_value < (target + estimated_fees.max(options.min_absolute_fee)) {
        Err(SelectionError::InsufficientFunds)
    } else {
        let waste: u64 = calculate_waste(
            inputs,
            &selected_inputs,
            &options,
            accumulated_value,
            accumulated_weight,
            estimated_fees,
        );
        Ok(SelectionOutput {
            selected_inputs,
            waste: WasteMetric(waste),
        })
    }
}

/// Perform Coinselection via First-In-First-Out algorithm.
/// Return NoSolutionFound, if no solution exists.
pub fn select_coin_fifo(
    inputs: &[OutputGroup],
    options: CoinSelectionOpt,
) -> Result<SelectionOutput, SelectionError> {
    let mut accumulated_value: u64 = 0;
    let mut accumulated_weight: u32 = 0;
    let mut selected_inputs: Vec<usize> = Vec::new();
    let mut estimated_fees: u64 = 0;

    // Sorting the inputs vector based on creation_sequence

    let mut sorted_inputs: Vec<_> = inputs.iter().enumerate().collect();

    sorted_inputs.sort_by_key(|(_, a)| a.creation_sequence);

    for (index, inputs) in sorted_inputs {
        estimated_fees = calculate_fee(accumulated_weight, options.target_feerate);
        if accumulated_value
            >= (options.target_value
                + estimated_fees.max(options.min_absolute_fee)
                + options.min_drain_value)
        {
            break;
        }
        accumulated_value += inputs.value;
        accumulated_weight += inputs.weight;
        selected_inputs.push(index);
    }
    if accumulated_value
        < options.target_value
            + estimated_fees.max(options.min_absolute_fee)
            + options.min_drain_value)
    {
        Err(SelectionError::InsufficientFunds)
    } else {
        let waste: u64 = calculate_waste(
            inputs,
            &selected_inputs,
            &options,
            accumulated_value,
            accumulated_weight,
            estimated_fees,
        );
        Ok(SelectionOutput {
            selected_inputs,
            waste: WasteMetric(waste),
        })
    }
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
    inputs: &[OutputGroup],
    options: CoinSelectionOpt,
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
    // waste =  weight*(target feerate - long term fee rate) + cost of change + excess
    // weight - total weight of selected inputs
    // cost of change - includes the fees paid on this transaction's change output plus the fees that will need to be paid to spend it later. If there is no change output, the cost is 0.
    // excess - refers to the difference between the sum of selected inputs and the amount we need to pay (the sum of output values and fees). There shouldn’t be any excess if there is a change output.

    let mut waste: u64 = 0;
    if let Some(long_term_feerate) = options.long_term_feerate {
        waste = (accumulated_weight as f32 * (options.target_feerate - long_term_feerate)).ceil()
            as u64;
    }
    if options.excess_strategy != ExcessStrategy::ToDrain {
        // Change is not created if excess strategy is ToFee or ToRecipient. Hence cost of change is added
        waste += (accumulated_value - (options.target_value + estimated_fee));
    } else {
        // Change is created if excess strategy is set to ToDrain. Hence 'excess' should be set to 0
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
    const CENT: f64 = 1000000.0;
    const COIN: f64 = 100000000.0;
    const RUN_TESTS: u32 = 100;

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
            long_term_feerate: Some(0.4),
            min_absolute_fee: 0,
            base_weight: 10,
            drain_weight: 50,
            drain_cost: 10,
            cost_per_input: 20,
            cost_per_output: 10,
            min_drain_value: 500, //CENT.round() as u64,
            excess_strategy: ExcessStrategy::ToDrain,
        }
    }
    fn setup_output_groups(value: Vec<u64>, weights: Vec<u32>) -> Vec<OutputGroup> {
        let mut inputs: Vec<OutputGroup> = Vec::new();
        for (i, j) in value.into_iter().zip(weights.into_iter()) {
            inputs.push(OutputGroup {
                value: i,
                weight: j,
                input_count: 1,
                is_segwit: false,
                creation_sequence: None,
            })
        }
        inputs
    }

    fn add_to_output_group(inputs: &mut Vec<OutputGroup>, value: Vec<u64>, weights: Vec<u32>) {
        for (i, j) in value.into_iter().zip(weights.into_iter()) {
            inputs.push(OutputGroup {
                value: i,
                weight: j,
                input_count: 1,
                is_segwit: false,
                creation_sequence: None,
            })
        }
    }
    #[test]
    fn test_bnb() {
        // Perform BNB selection of set of test values.
    }

    fn test_successful_selection() {
        let mut inputs = setup_basic_output_groups();
        let mut options = setup_options(2500);
        let mut result = select_coin_srd(&inputs, options);
        assert!(result.is_ok());
        let mut selection_output = result.unwrap();
<<<<<<< HEAD
        assert!(!selection_output.selected_inputs.is_empty());

        inputs = setup_output_groups_withsequence();
        options = setup_options(500);
        result = select_coin_fifo(&inputs, options);
        assert!(result.is_ok());
        selection_output = result.unwrap();
=======
>>>>>>> b3e707d (Removed a redundant utility function. Substituted its function with the existing utility function)
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
    fn test_core_knapsack_vectors() {
        let mut inputs_verify: Vec<usize> = Vec::new();
        for i in 0..RUN_TESTS {
            // Test if Knapsack retruns an Error
            let mut inputs: Vec<OutputGroup> = Vec::new();
            let mut options = setup_options(1000);
            let mut result = select_coin_knapsack(&inputs, options);
            assert!(matches!(result, Err(SelectionError::NoSolutionFound)));

            // Adding 2 CENT and 1 CENT to the wallet and testing if knapsack can select the two inputs for a 3 CENT Output
            // Adding 0.001 CENT to the inputs to account for fees.
            inputs = setup_output_groups(
                vec![(2.001 * CENT).round() as u64, (1.001 * CENT).round() as u64],
                vec![130, 100],
            );
            options = setup_options((3.0 * CENT).round() as u64);
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
            // Adding 0.001 CENT to the inputs to account for fees
            add_to_output_group(
                &mut inputs,
                vec![
                    (5.001 * CENT).round() as u64,
                    (10.001 * CENT).round() as u64,
                    (20.001 * CENT).round() as u64,
                ],
                vec![100, 1, 5],
            );
            // Testing if knapsack can select 4 inputs (2,5,10,20) CENTS to make 37 CENTS
            options = setup_options((37.0 * CENT).round() as u64);
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
            options = setup_options((38.0 * CENT).round() as u64);
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
            options = setup_options((34.0 * CENT).round() as u64);
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
            options = setup_options((7.0 * CENT).round() as u64);
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
            options = setup_options((8.0 * CENT).round() as u64);
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
            options = setup_options((10.0 * CENT).round() as u64);
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
            inputs = setup_output_groups(
                vec![
                    (6.001 * CENT).round() as u64,
                    (7.001 * CENT).round() as u64,
                    (8.001 * CENT).round() as u64,
                    (20.001 * CENT).round() as u64,
                    (30.001 * CENT).round() as u64,
                ],
                vec![100, 200, 100, 10, 5],
            );
            // Testing if Knapsack retruns an Error while trying to select inputs totalling 72 CENTS
            options = setup_options((72.0 * CENT).round() as u64);
            result = select_coin_knapsack(&inputs, options);
            assert!(matches!(result, Err(SelectionError::NoSolutionFound)));
            // Testing if knapsack can select 3 input (6,7,8) CENTS to make 16 CENTS
            options = setup_options((16.0 * CENT).round() as u64);
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
            // Adding 0.001 CENT to the input to account for fees
            add_to_output_group(&mut inputs, vec![(5.001 * CENT).round() as u64], vec![10]);
            // Testing if knapsack can select 3 input (5,6,7) CENTS to make 16 CENTS
            options = setup_options((16.0 * CENT).round() as u64);
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
        }
    }
    #[test]
    fn test_srd() {
        test_successful_selection();
        test_insufficient_funds();
    }

    #[test]
    fn test_knapsack() {
        test_core_knapsack_vectors();
    }

    #[test]
    fn test_fifo() {
        test_successful_selection();
        test_insufficient_funds();
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
}
