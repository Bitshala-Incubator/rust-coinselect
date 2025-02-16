use crate::types::{
    CoinSelectionOpt, EffectiveValue, ExcessStrategy, OutputGroup, SelectionError, Weight,
};
use std::{collections::HashSet, fmt};

#[inline]
pub fn calculate_waste(
    options: &CoinSelectionOpt,
    accumulated_value: u64,
    accumulated_weight: u64,
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
        waste += accumulated_value
            .saturating_sub(options.target_value)
            .saturating_sub(estimated_fee);
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
) -> u64 {
    let mut accumulated_weight: u64 = 0;
    for &(index, _value, weight) in smaller_coins {
        if selected_inputs.contains(&index) {
            accumulated_weight += weight;
        }
    }
    accumulated_weight
}

impl fmt::Display for SelectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SelectionError::NonPositiveFeeRate => write!(f, "Negative fee rate"),
            SelectionError::AbnormallyHighFeeRate => write!(f, "Abnormally high fee rate"),
            SelectionError::InsufficientFunds => write!(f, "The Inputs funds are insufficient"),
            SelectionError::NoSolutionFound => write!(f, "No solution could be derived"),
        }
    }
}

impl std::error::Error for SelectionError {}

type Result<T> = std::result::Result<T, SelectionError>;

#[inline]
pub fn calculate_fee(weight: u64, rate: f32) -> Result<u64> {
    if rate <= 0.0 {
        Err(SelectionError::NonPositiveFeeRate)
    } else if rate > 1000.0 {
        Err(SelectionError::AbnormallyHighFeeRate)
    } else {
        Ok((weight as f32 * rate).ceil() as u64)
    }
}

/// Returns the effective value of the `OutputGroup`, which is the actual value minus the estimated fee.
#[inline]
pub fn effective_value(output: &OutputGroup, feerate: f32) -> Result<u64> {
    Ok(output
        .value
        .saturating_sub(calculate_fee(output.weight, feerate)?))
}

/// Returns the weights of data in transaction other than the list of inputs that would be selected.
pub fn calculate_base_weight_btc(output_weight: u64) -> u64 {
    // VERSION_SIZE: 4 bytes - 16 WU
    // SEGWIT_MARKER_SIZE: 2 bytes - 2 WU
    // NUM_INPUTS_SIZE: 1 byte - 4 WU
    // NUM_OUTPUTS_SIZE: 1 byte - 4 WU
    // NUM_WITNESS_SIZE: 1 byte - 1 WU
    // LOCK_TIME_SIZE: 4 bytes - 16 WU
    // OUTPUT_VALUE_SIZE: variable

    // Total default: (16 + 2 + 4 + 4 + 1 + 16 = 43 WU + variable) WU
    // Source - https://docs.rs/bitcoin/latest/src/bitcoin/blockdata/transaction.rs.html#599-602
    output_weight + 43
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CoinSelectionOpt, ExcessStrategy, OutputGroup, SelectionError};

    fn setup_options(target_value: u64) -> CoinSelectionOpt {
        CoinSelectionOpt {
            target_value,
            target_feerate: 0.4, // Simplified feerate
            long_term_feerate: Some(0.4),
            min_absolute_fee: 0,
            base_weight: 10,
            change_weight: 50,
            change_cost: 10,
            avg_input_weight: 20,
            avg_output_weight: 10,
            min_change_value: 500,
            excess_strategy: ExcessStrategy::ToChange,
        }
    }

    /// Tests the fee calculation function with various input scenarios.
    /// Fee calculation is critical for coin selection as it determines the effective value
    /// of each UTXO after accounting for the cost to spend it.
    ///
    /// Test vectors cover:
    /// - Normal fee calculation with positive rate
    /// - Error case with negative fee rate
    /// - Error case with abnormally high fee rate (>1000 sat/vB)
    /// - Edge case with zero fee rate
    #[test]
    fn test_calculate_fee() {
        struct TestVector {
            weight: u64,
            fee: f32,
            output: Result<u64>,
        }

        let test_vectors = [
            TestVector {
                weight: 60,
                fee: 5.0,
                output: Ok(300),
            },
            TestVector {
                weight: 60,
                fee: -5.0,
                output: Err(SelectionError::NonPositiveFeeRate),
            },
            TestVector {
                weight: 60,
                fee: 1001.0,
                output: Err(SelectionError::AbnormallyHighFeeRate),
            },
            TestVector {
                weight: 60,
                fee: 0.0,
                output: Err(SelectionError::NonPositiveFeeRate),
            },
        ];

        for vector in test_vectors {
            let result = calculate_fee(vector.weight, vector.fee);
            match result {
                Ok(val) => {
                    assert_eq!(val, vector.output.unwrap())
                }
                Err(err) => {
                    let output = vector.output.err();
                    assert_eq!(err, output.unwrap());
                }
            }
        }
    }

    /// Tests the effective value calculation which determines the actual spendable amount
    /// of a UTXO after subtracting the fee required to spend it.
    ///
    /// Effective value is crucial for coin selection as it helps:
    /// - Avoid selecting UTXOs that cost more in fees than their value
    /// - Compare UTXOs based on their true spendable amount
    /// - Calculate the actual amount available for spending
    ///
    /// Test vectors cover:
    /// - Edge case where fees exceed UTXO value
    /// - Normal case with positive effective value
    /// - Error cases with invalid fee rates
    /// - Large value UTXO calculations
    #[test]
    fn test_effective_value() {
        struct TestVector {
            output: OutputGroup,
            feerate: f32,
            result: Result<u64>,
        }

        let test_vectors = [
            // Value minus weight would be less Than Zero but will return zero because of saturating_subtraction for u64
            TestVector {
                output: OutputGroup {
                    value: 100,
                    weight: 101,
                    input_count: 1,
                    creation_sequence: None,
                },
                feerate: 1.0,
                result: Ok(0),
            },
            // Value greater than zero
            TestVector {
                output: OutputGroup {
                    value: 100,
                    weight: 99,
                    input_count: 1,
                    creation_sequence: None,
                },
                feerate: 1.0,
                result: Ok(1),
            },
            // Test negative fee rate return appropriate error
            TestVector {
                output: OutputGroup {
                    value: 100,
                    weight: 99,
                    input_count: 1,
                    creation_sequence: None,
                },
                feerate: -1.0,
                result: Err(SelectionError::NonPositiveFeeRate),
            },
            // Test very high fee rate
            TestVector {
                output: OutputGroup {
                    value: 100,
                    weight: 99,
                    input_count: 1,
                    creation_sequence: None,
                },
                feerate: 2000.0,
                result: Err(SelectionError::AbnormallyHighFeeRate),
            },
            // Test high value
            TestVector {
                output: OutputGroup {
                    value: 100_000_000_000,
                    weight: 10,
                    input_count: 1,
                    creation_sequence: None,
                },
                feerate: 1.0,
                result: Ok(99_999_999_990),
            },
        ];

        for vector in test_vectors {
            let effective_value = effective_value(&vector.output, vector.feerate);

            match effective_value {
                Ok(val) => {
                    assert_eq!(Ok(val), vector.result)
                }
                Err(err) => {
                    assert_eq!(err, vector.result.unwrap_err());
                }
            }
        }
    }

    /// Tests the waste metric calculation which helps optimize coin selection.
    /// Waste represents the cost of creating a change output plus any excess amount
    /// that goes to fees or is added to recipient outputs.
    ///
    /// The waste metric considers:
    /// - Long-term vs current fee rates
    /// - Cost of creating change outputs
    /// - Excess amounts based on selected strategy (fee/change/recipient)
    ///
    /// Test vectors cover:
    /// - Change output creation (ToChange strategy)
    /// - Fee payment (ToFee strategy)
    /// - Insufficient funds scenario
    #[test]
    fn test_calculate_waste() {
        struct TestVector {
            options: CoinSelectionOpt,
            accumulated_value: u64,
            accumulated_weight: u64,
            estimated_fee: u64,
            result: u64,
        }

        let options = setup_options(100).clone();
        let test_vectors = [
            // Test for excess strategy to drain(change output)
            TestVector {
                options: options.clone(),
                accumulated_value: 1000,
                accumulated_weight: 50,
                estimated_fee: 20,
                result: options.change_cost,
            },
            // Test for excess strategy to miners
            TestVector {
                options: CoinSelectionOpt {
                    excess_strategy: ExcessStrategy::ToFee,
                    ..options
                },
                accumulated_value: 1000,
                accumulated_weight: 50,
                estimated_fee: 20,
                result: 880,
            },
            // Test accumulated_value minus target_value < 0
            TestVector {
                options: CoinSelectionOpt {
                    target_value: 1000,
                    excess_strategy: ExcessStrategy::ToFee,
                    ..options
                },
                accumulated_value: 200,
                accumulated_weight: 50,
                estimated_fee: 20,
                result: 0,
            },
        ];

        for vector in test_vectors {
            let waste = calculate_waste(
                &vector.options,
                vector.accumulated_value,
                vector.accumulated_weight,
                vector.estimated_fee,
            );

            assert_eq!(waste, vector.result)
        }
    }
}
