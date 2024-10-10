use crate::types::{CoinSelectionOpt, EffectiveValue, ExcessStrategy, OutputGroup, Weight};
use std::collections::HashSet;

#[inline]
pub fn calculate_waste(
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
    if options.excess_strategy != ExcessStrategy::ToChange {
        // Change is not created if excess strategy is ToFee or ToRecipient. Hence cost of change is added
        waste += accumulated_value - (options.target_value + estimated_fee);
    } else {
        // Change is created if excess strategy is set to ToChange. Hence 'excess' should be set to 0
        waste += options.change_cost;
    }
    waste
}

/// `adjusted_target` is the target value plus the estimated fee.
///
/// `smaller_coins` is a slice of pairs where the `usize` refers to the index of the `OutputGroup` in the provided inputs.
/// This slice should be sorted in descending order by the value of each `OutputGroup`, with each value being less than `adjusted_target`.
pub fn calculate_accumulated_weight(
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

#[inline]
pub fn calculate_fee(weight: u32, rate: f32) -> u64 {
    (weight as f32 * rate).ceil() as u64
}

/// Returns the effective value of the `OutputGroup`, which is the actual value minus the estimated fee.
#[inline]
pub fn effective_value(output: &OutputGroup, feerate: f32) -> u64 {
    output
        .value
        .saturating_sub(calculate_fee(output.weight, feerate))
}
