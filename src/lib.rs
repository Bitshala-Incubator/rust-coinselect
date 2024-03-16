#![allow(unused)]

//! A blockchain-agnostic Rust Coinselection library
/// A [`OutputGroup`] represents an input candidate for Coinselection. This can either be a
/// single UTXO, or a group of UTXOs that should be spent together.
/// The library user is responsible for crafting this structure correctly. Incorrect representation of this
/// structure will cause incorrect selection result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Clone, Copy, Debug, PartialEq)]
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

/// Perform Coinselection via Branch And Bound algorithm.
pub fn select_coin_bnb(
    inputs: Vec<OutputGroup>,
    opitons: CoinSelectionOpt,
    excess_strategy: ExcessStrategy,
) -> Result<SelectionOutput, SelectionError> {
    unimplemented!()
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
    /* Using the value of the input as an identifier for the selected inputs, as the index of the vec<outputgroup> can't be used because the vector itself is sorted first. Ideally, txid of the input can serve as an unique identifier */
    let mut accumulated_value: u64 = 0;
    let mut accumulated_weight: u32 = 0;
    let mut selected_inputs: Vec<usize> = Vec::new();
    let mut estimated_fees: u64 = 0;

    // Sorting the inputs vector based on creation_sequence

    let mut sortedinputs = inputs.clone();
    sortedinputs.sort_by_key(|a| a.creation_sequence);

    for i in sortedinputs.iter() {
        estimated_fees = (accumulated_weight as f32 * options.target_feerate).ceil() as u64;
        if accumulated_value
            >= (options.target_value
                + options.target_value
                + estimated_fees
                + options.min_drain_value)
        {
            break;
        }
        accumulated_value += i.value;
        accumulated_weight += i.weight;
        if let Some(index) = inputs.iter().position(|x| x == i) {
            selected_inputs.push(index);
        } else {
            break;
        }
    }
    if accumulated_value < options.target_value + estimated_fees + options.min_drain_value {
        Err(SelectionError::NoSolutionFound)
    } else {
        let mut waste_score: u64 = 0;
        if options.excess_strategy == ExcessStrategy::ToDrain {
            let waste_score: u64 = calculate_waste(
                &inputs,
                &selected_inputs,
                &options,
                accumulated_value,
                accumulated_weight,
                estimated_fees,
            );
        } else {
            waste_score = 0;
        }
        Ok(SelectionOutput {
            selected_inputs,
            waste: WasteMetric(waste_score),
        })
    }
}

/// Perform Coinselection via Single Random Draw.
/// Return NoSolutionFound, if no solution exists.
pub fn select_coin_srd(
    inputs: Vec<OutputGroup>,
    opitons: CoinSelectionOpt,
    excess_strategy: ExcessStrategy,
) -> Result<SelectionOutput, SelectionError> {
    unimplemented!()
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

pub fn calc_waste_metric(
    inp_weight: u32,
    target_feerate: f32,
    longterm_feerate: Option<f32>,
    drain_weight: u32,
    totalvalue: u64,
    target_value: u64,
) -> u64 {
    match longterm_feerate {
        Some(fee) => {
            let change: f32 = drain_weight as f32 * fee;
            let excess: u64 = totalvalue - target_value;

            (inp_weight as f32 * (target_feerate - fee) + change + excess as f32).ceil() as u64
        }
        None => {
            let waste_score: u64 = 0;
            waste_score
        }
    }
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
            drain_cost: 10,
            cost_per_input: 20,
            cost_per_output: 10,
            min_drain_value: 500,
            excess_strategy: ExcessStrategy::ToDrain,
        }
    }

    #[test]
    fn test_bnb() {
        // Perform BNB selection of set of test values.
    }

    fn test_successful_selection() {
        let inputs = setup_basic_output_groups();
        let options = setup_options(2500);
        let result = select_coin_srd(&inputs, options);
        assert!(result.is_ok());
        let selection_output = result.unwrap();
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
        test_successful_selection();
        test_insufficient_funds();
    }

    #[test]
    fn test_knapsack() {
        // Perform Knapsack selection of set of test values.
    }

    #[test]
    fn test_successful_fifo_selection() {
        let inputs = setup_fifo_output_groups();
        let options = setup_options(500); // Seting up target value such that excess exists
        let result = select_coin_fifo(inputs, options);
        let selection_output = result.unwrap();
        assert!(!selection_output.selected_inputs.is_empty());
    }
    fn test_fifo() {
        // Test for Empty Values
        let inputs = vec![];
        let options = CoinSelectionOpt {
            target_value: 0,
            target_feerate: 0.0,
            long_term_feerate: None,
            min_absolute_fee: 0,
            base_weight: 0,
            drain_weight: 0,
            spend_drain_weight: 0,
            min_drain_value: 0,
        };
        let excess_strategy = ExcessStrategy::ToFee;
        let result = select_coin_fifo(inputs, options, excess_strategy);
        // Check if select_coin_fifo() returns the correct type
        assert!(result.is_ok());
    }

    #[test]
    fn test_lowestlarger() {
        // Perform LowestLarger selection of set of test values.
    }
}
