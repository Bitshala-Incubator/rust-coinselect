#![allow(unused)]

//! A blockchain-agnostic Rust Coinselection library

use rand::{seq::SliceRandom, thread_rng};

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
    pub drain_cost: u64, 

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

/// Calculated waste for a specific selection.
/// This is used to compare various selection algorithm and find the most
/// optimizewd solution, represented by least [WasteMetric] value.
#[derive(Debug)]
pub struct WasteMetric(u64);

/// The result of selection algorithm
pub struct SelectionOutput {
    /// The selected inputs
    pub selected_inputs: Vec<usize>,
    /// The waste amount, for the above inputs
    pub waste: WasteMetric,
}

/// Perform Coinselection via Branch And Bound algorithm.
pub fn select_coin_bnb(
    inputs: Vec<OutputGroup>,
    options: CoinSelectionOpt,
    excess_strategy: ExcessStrategy,
) -> Result<SelectionOutput, SelectionError> {
    unimplemented!()
}

/// Perform Coinselection via Knapsack solver.
pub fn select_coin_knapsack(
    inputs: Vec<OutputGroup>,
    options: CoinSelectionOpt,
    excess_strategy: ExcessStrategy,
) -> Result<SelectionOutput, SelectionError> {
    unimplemented!()
}

/// Perform Coinselection via Lowest Larger algorithm.
/// Return NoSolutionFound, if no solution exists.
pub fn select_coin_lowestlarger(
    inputs: Vec<OutputGroup>,
    options: CoinSelectionOpt,
    excess_strategy: ExcessStrategy,
) -> Result<SelectionOutput, SelectionError> {
    unimplemented!()
}

/// Perform Coinselection via First-In-First-Out algorithm.
/// Return NoSolutionFound, if no solution exists.
pub fn select_coin_fifo(
    inputs: Vec<OutputGroup>,
    options: CoinSelectionOpt,
    excess_strategy: ExcessStrategy,
) -> Result<SelectionOutput, SelectionError> {
    unimplemented!()
}

/// Perform Coinselection via Single Random Draw.
/// Return NoSolutionFound, if no solution exists.
pub fn select_coin_srd(
    inputs: Vec<OutputGroup>,
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

    for (index, input) in randomized_inputs {
        selected_inputs.push(index);
        accumulated_value += input.value;
        accumulated_weight += input.weight;
        input_counts += input.input_count;

        estimated_fee = (accumulated_weight as f32 * options.target_feerate).ceil() as u64;

        if accumulated_value >= options.target_value + options.min_drain_value + estimated_fee {
            break;
        }
    }

    if accumulated_value < options.target_value + options.min_drain_value + estimated_fee {
        return Err(SelectionError::NoSolutionFound);
    }
    // accumulated_weight += weightof(input_counts)?? TODO
    let waste = calculate_waste(&inputs, &selected_inputs, &options, accumulated_value, accumulated_weight, estimated_fee);

    Ok(SelectionOutput {
        selected_inputs,
        waste: WasteMetric(waste),
    })
}

/// The Global Coinselection API that performs all the algorithms and proudeces result with least [WasteMetric].
/// At least one selection solution should be found.
pub fn select_coin(
    inputs: Vec<OutputGroup>,
    options: CoinSelectionOpt,
    excess_strategy: ExcessStrategy,
) -> Result<SelectionOutput, SelectionError> {
    unimplemented!()
}

#[inline]
fn calculate_waste(inputs: &Vec<OutputGroup>, selected_inputs: &Vec<usize>, options: &CoinSelectionOpt, accumulated_value: u64, accumulated_weight: u32, estimated_fee: u64) -> u64 {
    let mut waste: u64 = 0;

    if let Some(long_term_feerate) = options.long_term_feerate {
        waste += (estimated_fee as f32 - selected_inputs.len() as f32 * long_term_feerate * accumulated_weight as f32).ceil() as u64;
    }

    if options.excess_strategy != ExcessStrategy::ToDrain {
        waste += accumulated_value - options.target_value - estimated_fee;
    } else {
        waste += options.drain_cost;
    }
    
    waste
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_bnb() {
        // Perform BNB selection of set of test values.
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
    fn test_lowestlarger() {
        // Perform LowestLarger selection of set of test values.
    }
}
