use crate::types::{CoinSelectionOpt, OutputGroup, SelectionError, SelectionOutput};

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

#[cfg(test)]
mod test {
    #[test]
    fn test_bnb() {
        // Perform BNB selection of set of test values.
    }
}
