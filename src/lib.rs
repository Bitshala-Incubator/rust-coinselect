#![allow(unused)]

//! A blockchain-agnostic Rust Coinselection library

use rand::{rngs::ThreadRng, seq::SliceRandom, thread_rng, Rng};
use std::cmp::Reverse;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::thread;

/// Represents an input candidate for Coinselection, either as a single UTXO or a group of UTXOs.
///
/// A [`OutputGroup`] can be a single UTXO or a group that should be spent together.
/// The library user must craft this structure correctly, as incorrect representation can lead to incorrect selection results.
#[derive(Debug, Clone, Copy)]
pub struct OutputGroup {
    /// Total value of the UTXO(s) that this [`WeightedValue`] represents.
    pub value: u64,
    /// Total weight of including these UTXO(s) in the transaction.
    ///
    /// The `txin` fields: `prevout`, `nSequence`, `scriptSigLen`, `scriptSig`, `scriptWitnessLen`,
    /// and `scriptWitness` should all be included.
    pub weight: u32,
    /// The total number of inputs; so we can calculate extra `varint` weight due to `vin` length changes.
    pub input_count: usize,
    /// Whether this [`OutputGroup`] contains at least one segwit spend.
    pub is_segwit: bool,
    /// Specifies the relative creation sequence for this group, used only for FIFO selection.
    ///
    /// Set to `None` if FIFO selection is not required. Sequence numbers are arbitrary indices that denote the relative age of a UTXO group among a set of groups.
    /// To denote the oldest UTXO group, assign it a sequence number of `Some(0)`.
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

    /// Estimate of cost of spending an input.
    pub cost_per_input: u64,

    /// Estimate of cost of spending the output.
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

/// Error Describing failure of a selection attempt, on any subset of inputs.
#[derive(Debug, PartialEq)]
pub enum SelectionError {
    InsufficientFunds,
    NoSolutionFound,
}

/// Measures the efficiency of input selection in satoshis, helping evaluate algorithms based on current and long-term fee rates.
///
/// WasteMetric strikes a balance between minimizing current transaction fees and overall fees during the wallet's lifetime.
/// In high fee rate environments, selecting fewer inputs reduces transaction fees.
/// In low fee rate environments, selecting more inputs reduces overall fees.
/// It compares various selection algorithms to find the most optimized solution, represented by the lowest [WasteMetric] value.
#[derive(Debug)]
pub struct WasteMetric(u64);

/// The result of selection algorithm.
#[derive(Debug)]
pub struct SelectionOutput {
    /// The selected input indices, refers to the indices of the inputs Slice Reference.
    pub selected_inputs: Vec<usize>,
    /// The waste amount, for the above inputs.
    pub waste: WasteMetric,
}
/// Struct for three arguments : target_for_match, match_range and target_feerate
///
/// Wrapped in a struct or else input for fn bnb takes too many arguments - 9/7
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
) -> Result<SelectionOutput, SelectionError> {
    let mut selected_inputs: Vec<usize> = vec![];

    /// Variable is mutable for decrement of bnb_tries for every iteration of fn bnb
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
///
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
    // Capping the number of iterations on the computation
    if *bnb_tries == 0 || depth >= inputs_in_desc_value.len() {
        return None;
    }
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
                        None // this may or may not be correct
                    }
                }
            }
        }
    }
}

/// Perform Coinselection via Knapsack solver.
type EffectiveValue = u64;
type Weight = u32;

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
        .map(|(index, output_group)| {
            (
                index,
                effective_value(output_group, options.target_feerate),
                output_group.weight,
            )
        })
        .collect::<Vec<_>>();
    smaller_coins.sort_by_key(|&(_, value, _)| Reverse(value));

    knap_sack(adjusted_target, &smaller_coins, inputs, options)
}

/// `adjusted_target` is the target value plus the estimated fee.
///
/// `smaller_coins` is a slice of pairs where the `usize` refers to the index of the `OutputGroup` in the provided inputs.
/// This slice should be sorted in descending order by the value of each `OutputGroup`, with each value being less than `adjusted_target`.
fn calculate_accumulated_weight(
    smaller_coins: &[(usize, EffectiveValue, Weight)],
    selected_inputs: &HashSet<usize>,
) -> u32 {
    let mut accumulated_weight: u32 = 0;
    for &(index, _value, weight) in smaller_coins {
        if selected_inputs.contains(&index) {
            accumulated_weight += weight;
        }
    }
    accumulated_weight
}

fn knap_sack(
    adjusted_target: u64,
    smaller_coins: &[(usize, EffectiveValue, Weight)],
    inputs: &[OutputGroup],
    options: CoinSelectionOpt,
) -> Result<SelectionOutput, SelectionError> {
    let mut selected_inputs: HashSet<usize> = HashSet::new();
    let mut accumulated_value: u64 = 0;
    let mut best_set: HashSet<usize> = HashSet::new();
    let mut best_set_value: u64 = u64::MAX;
    let mut rng = thread_rng();
    for i in 1..=1000 {
        for pass in 1..=2 {
            for &(index, value, weight) in smaller_coins {
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

/// Performs coin selection using the Lowest Larger algorithm.
///
/// Returns `NoSolutionFound` if no solution exists.
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

/// Performs coin selection using the First-In-First-Out (FIFO) algorithm.
///
/// Returns `NoSolutionFound` if no solution is found.
pub fn select_coin_fifo(
    inputs: &[OutputGroup],
    options: CoinSelectionOpt,
) -> Result<SelectionOutput, SelectionError> {
    let mut accumulated_value: u64 = 0;
    let mut accumulated_weight: u32 = 0;
    let mut selected_inputs: Vec<usize> = Vec::new();
    let mut estimated_fees: u64 = 0;

    // Sorting the inputs vector based on creation_sequence
    let mut sorted_inputs: Vec<_> = inputs
        .iter()
        .enumerate()
        .filter(|(_, og)| og.creation_sequence.is_some())
        .collect();

    sorted_inputs.sort_by(|a, b| a.1.creation_sequence.cmp(&b.1.creation_sequence));

    let mut inputs_without_sequence: Vec<_> = inputs
        .iter()
        .enumerate()
        .filter(|(_, og)| og.creation_sequence.is_none())
        .collect();

    sorted_inputs.extend(inputs_without_sequence);

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
        < (options.target_value
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

/// Performs coin selection using a single random draw.
///
/// Returns `NoSolutionFound` if no solution is found.
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

/// The global coin selection API that applies all algorithms and produces the result with the lowest [WasteMetric].
///
/// At least one selection solution should be found.
type CoinSelectionFn =
    fn(&[OutputGroup], CoinSelectionOpt) -> Result<SelectionOutput, SelectionError>;

#[derive(Debug)]
struct SharedState {
    result: Result<SelectionOutput, SelectionError>,
    any_success: bool,
}

pub fn select_coin(
    inputs: &[OutputGroup],
    options: CoinSelectionOpt,
) -> Result<SelectionOutput, SelectionError> {
    let algorithms: Vec<CoinSelectionFn> = vec![
        select_coin_bnb,
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

/// Returns the effective value of the `OutputGroup`, which is the actual value minus the estimated fee.
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
    const RUN_TESTS_SLIM: u32 = 10;
    const RANDOM_REPEATS: u32 = 5;

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
            OutputGroup {
                value: 1500,
                weight: 150,
                input_count: 1,
                is_segwit: false,
                creation_sequence: None,
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
    fn test_core_knapsack_vectors() {
        let mut inputs_verify: Vec<usize> = Vec::new();
        for i in 0..RUN_TESTS {
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
                drain_weight: 50,
                drain_cost: 10,
                cost_per_input: 20,
                cost_per_output: 10,
                min_drain_value: (0.05 * CENT).round() as u64, // Setting minimum drain value = 0.05 CENT. This will make the algorithm to avoid creating small change.
                excess_strategy: ExcessStrategy::ToDrain,
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
            for i in (0..676) {
                // Populate the vectors with the same value 'amt' and weight = 23 for 676 times
                // Using 676 as (old MAX_STANDARD_TX_SIZE = 100000)/(148 bytes per input) = 676
                input_value.push(amt);
                input_weight.push(23);
            }
            let mut inputs = knapsack_setup_output_groups(input_value, input_weight, 0.34);
            // Setting the selection target to 2000 sats
            let mut options = knapsack_setup_options(2000, 0.34);
            // performing the assertion operation 10 times
            for j in 0..RUN_TESTS_SLIM {
                if let Ok(result) = select_coin_knapsack(&inputs, options) {
                    if let Some(amt_in_inputs) = inputs.first() {
                        // Checking if the (input's value) - 2000 is less than CENT
                        // If so, more than one input is required to meet the selection target of 2000 sats
                        if amt_in_inputs.value.checked_sub(2000) < Some(CENT as u64) {
                            // calculating the no.of inputs that will be required to meet the selection target of 2000 sats
                            let mut return_size = ((2000.0) / amt as f64).ceil();
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
        for i in 0..=100 {
            // Populate the vectors with the same value, COIN = 100000000 sats, and weight = 23 for 100 times (to create 100 identical inputs)
            input_value.push(COIN as u64);
            input_weight.push(23);
        }
        // Setting up inputs
        let mut inputs = knapsack_setup_output_groups(input_value, input_weight, 0.34);
        // Setting the selection target to 50*COIN sats
        let mut options = knapsack_setup_options((50.0 * COIN).round() as u64, 0.34);
        let mut selected_input_1: Vec<usize> = Vec::new();
        let mut selected_input_2: Vec<usize> = Vec::new();
        for j in 0..RUN_TESTS {
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
        /* Testing if the algorithm can randomly select inputs to make 160,000,000 SATS.

        The test checks if the algorithm can pick a random sample of inputs (from a set of 105) to make 160,000,000 SATS.
        When choosing 1 from 100 identical inputs (1 COIN), there is a 1% chance of selecting the same input twice.
        To evaluate randomness, we limit our check to 5 trials: if the algorithm picks the same set of inputs 5 times,
        we conclude that the algorithm isn't random enough. */
        let mut options = knapsack_setup_options(((60.0 * CENT) + COIN).round() as u64, 0.34);
        let mut fails = 0;
        for k in 0..RUN_TESTS {
            for l in 0..RANDOM_REPEATS {
                if let Ok(result) = select_coin_knapsack(&inputs, options) {
                    selected_input_1.clone_from(&result.selected_inputs);
                }
                if let Ok(result) = select_coin_knapsack(&inputs, options) {
                    selected_input_2.clone_from(&result.selected_inputs);
                }
                if selected_input_1 == selected_input_2 {
                    fails += 1;
                }
            }
            assert_ne!(fails, RANDOM_REPEATS);
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
    fn test_bnb() {
        test_bnb_solution();
        test_bnb_no_solution();
    }

    #[test]
    fn test_fifo() {
        test_successful_selection();
        test_insufficient_funds();
    }

    #[test]
    fn test_fifo_with_none_and_sequence() {
        let inputs = vec![
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
                creation_sequence: None, // No sequence
            },
            OutputGroup {
                value: 3000,
                weight: 300,
                input_count: 1,
                is_segwit: false,
                creation_sequence: Some(0), // Oldest UTXO
            },
            OutputGroup {
                value: 1500,
                weight: 150,
                input_count: 1,
                is_segwit: false,
                creation_sequence: None, // No sequence
            },
            OutputGroup {
                value: 2500,
                weight: 250,
                input_count: 1,
                is_segwit: false,
                creation_sequence: Some(5), // Newer UTXO
            },
        ];

        let options = CoinSelectionOpt {
            target_value: 4500,
            target_feerate: 0.1,
            long_term_feerate: None,
            min_absolute_fee: 10,
            base_weight: 100,
            drain_weight: 50,
            drain_cost: 0,
            cost_per_input: 10,
            cost_per_output: 5,
            min_drain_value: 0,
            excess_strategy: ExcessStrategy::ToFee,
        };

        let result = select_coin_fifo(&inputs, options);

        assert!(result.is_ok());

        let selection_output = result.unwrap();
        // It should prioritize the ones with sequences: (3000, 1000, 2500) and then fall back on inputs without sequences
        assert_eq!(selection_output.selected_inputs.len(), 3); // Should select 3 inputs
        assert_eq!(selection_output.selected_inputs, vec![2, 0, 4]); // These are inputs with sequences

        assert!(selection_output.waste.0 < 10000); // Check waste does not exceed an arbitrary limit.
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
