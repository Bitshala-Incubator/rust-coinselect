#![allow(unused)]

//! A blockchain-agnostic Rust Coinselection library

use rand::{seq::SliceRandom, thread_rng , Rng};
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
    let mut selected_inputs : Vec<usize> =  vec![] ; 
    let bnb_tries = 1000000 ; 
    let bnb_selected_coin = bnb(&desce_input(inputs),&mut selected_inputs , 0 , 0 , bnb_tries , &options);
    if bnb_selected_coin.is_none() {
       return  select_coin_srd(inputs, options) ;
    }else {
        let selected_coin = bnb_selected_coin.unwrap();
        let accumulated_value :u64= selected_coin.iter().fold(0, |acc, &i| acc + inputs[i].value);
        let accumulated_weight : u32  = selected_coin.iter().fold(0, |acc, &i| acc + inputs[i].weight);
        let estimated_fee = 0 ;
        let waste = calculate_waste(inputs,&selected_inputs,&options,accumulated_value,accumulated_weight,estimated_fee,);
        let selection_output =  SelectionOutput {
             selected_inputs: selected_coin ,
             waste : WasteMetric(waste) 
        } ;
        return Ok(selection_output);
    }
   
}

/// Return empty vec if no solutions are found
// changing the selected_inputs : &[usize] -> &mut Vec<usize> 
fn bnb(
    inputs_in_desc_value: &[(usize, OutputGroup)],
    selected_inputs: &mut Vec<usize>,
    effective_values : u64,
    depth: usize,
    bnp_tries: u32,
    options: &CoinSelectionOpt
) -> option::Option<Vec<usize>> {
    let target_for_match = options.target_value + (((options.base_weight as f32) * options.target_feerate).ceil() as u64) + options.cost_per_output ; // here we include the CostPerOutput  
    let match_range = options.cost_per_input + options.cost_per_output; 
    if effective_values > target_for_match + match_range {
        return None;
    } else if effective_values >= target_for_match {
        return Some(selected_inputs.to_vec());
    } else if bnp_tries <= 0 {
        return None;
    } else if depth >= inputs_in_desc_value.len() {
        return None;
    } else {
        if generate_random_bool()==true 
        {
            // exploring the inclusion branch 
            // first include then omit
            let new_effective_values = effective_values + effective_value(&inputs_in_desc_value[depth].1 , options.target_feerate );
            selected_inputs.push(inputs_in_desc_value[depth].0); 
            let with_this = bnb(inputs_in_desc_value , selected_inputs , 
                new_effective_values , depth+1, bnp_tries-1, options) ;
                if with_this.is_some() 
                {
                    return with_this;
                }else 
                {
                    selected_inputs.pop();  //poping out the selected utxo if it does not fit 
                    let without_this = bnb(inputs_in_desc_value , selected_inputs , effective_values , depth+1, bnp_tries-1, options) ;
                    if without_this.is_some() 
                    {
                        return without_this;
                    } else {
                        return None ; // this may or may not be correct 
                    }
                }
        } else
         {
            //exploring the omitting branch
            //first omit then include 
            let without_this = bnb(inputs_in_desc_value , selected_inputs , effective_values , depth+1, bnp_tries-1, options) ;
            if without_this.is_some() {
                return without_this;
            } 
            else
             {
                let new_effective_values = effective_values + effective_value(&inputs_in_desc_value[depth].1 , options.target_feerate );
                selected_inputs.push(inputs_in_desc_value[depth].0);
                let with_this = bnb(inputs_in_desc_value , selected_inputs , 
                    new_effective_values , depth+1, bnp_tries-1, options) ;
                    if with_this.is_some() 
                    {
                        return with_this;
                    }
                    else 
                    {
                        selected_inputs.pop(); // poping out the selected utxo if it does not fit 
                        return None ; // this may or may not be correct}
                    }

              }
        }    
    }
 }



/// Perform Coinselection via Knapsack solver.
pub fn select_coin_knapsack(
    inputs: &[OutputGroup],
    options: CoinSelectionOpt,
) -> Result<SelectionOutput, SelectionError> {
    unimplemented!()
}

/// adjusted_target should be target value plus estimated fee
/// smaller_coins is a slice of pair where the usize refers to the index of the OutputGroup in the inputs given
/// smaller_coins should be sorted in descending order based on the value of the OutputGroup, and every OutputGroup value should be less than adjusted_target
fn knap_sack(
    adjusted_target: u64,
    smaller_coins: &[(usize, OutputGroup)],
) -> Result<SelectionOutput, SelectionError> {
    unimplemented!()
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

fn generate_random_bool() -> bool {
    // Create a random number generator
    let mut rng = rand::thread_rng();
    
    // Generate a random boolean value
    rng.gen()
}
pub fn desce_input<'a>(inputs: &'a [OutputGroup]) -> Vec<(usize, OutputGroup)> {
    let mut sorted_inputs: Vec<(usize, OutputGroup)> = inputs
        .iter()
        .enumerate()
        .map(|(index, input)| (index, input.clone()))
        .collect();

    sorted_inputs.sort_by_key(|(_, input)| std::cmp::Reverse(input.value));
    sorted_inputs
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

    #[test]
    fn test_bnb() {
        // Perform BNB selection of set of test values.
        let values = [
        OutputGroup {
            value: 10000000,
            weight: 100,
            input_count: 1,
            is_segwit: false,
            creation_sequence: Some(1),
        },
        OutputGroup {
            value: 5000000,
            weight: 200,
            input_count: 1,
            is_segwit: false,
            creation_sequence: Some(5000),
        },
        OutputGroup {
            value: 9000000,
            weight: 300,
            input_count: 1,
            is_segwit: false,
            creation_sequence: Some(1001),
        },
        OutputGroup {
            value: 270,
            weight: 10,
            input_count: 1,
            is_segwit: false,
            creation_sequence: Some(1000),
        }
    ];
    let opt = setup_options(14000000); 
    let ans = select_coin_bnb(&values, opt);
    assert!(ans.is_ok());
    assert!(!ans.unwrap().selected_inputs.contains(&0));  
    // as 10000000 should not be included in the selection
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
        test_successful_selection();
        test_insufficient_funds();
    }

    #[test]
    fn test_knapsack() {
        // Perform Knapsack selection of set of test values.
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
