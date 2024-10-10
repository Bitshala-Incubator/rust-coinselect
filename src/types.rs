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
    /// Additional weight if we include the change output.
    pub change_weight: u32,

    /// Weight of spending the change output in the future.
    pub change_cost: u64,

    /// Estimate of cost of spending an input.
    pub cost_per_input: u64,

    /// Estimate of cost of spending the output.
    pub cost_per_output: u64,

    /// Minimum value allowed for a change output.
    pub min_change_value: u64,

    /// Strategy to use the excess value other than fee and target
    pub excess_strategy: ExcessStrategy,
}

/// Strategy to decide what to do with the excess amount.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExcessStrategy {
    ToFee,
    ToRecipient,
    ToChange,
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
pub struct WasteMetric(pub u64);

/// The result of selection algorithm.
#[derive(Debug)]
pub struct SelectionOutput {
    /// The selected input indices, refers to the indices of the inputs Slice Reference.
    pub selected_inputs: Vec<usize>,
    /// The waste amount, for the above inputs.
    pub waste: WasteMetric,
}

/// Perform Coinselection via Knapsack solver.
pub type EffectiveValue = u64;
pub type Weight = u32;

/// The global coin selection API that applies all algorithms and produces the result with the lowest [WasteMetric].
///
/// At least one selection solution should be found.
pub type CoinSelectionFn =
    fn(&[OutputGroup], CoinSelectionOpt) -> Result<SelectionOutput, SelectionError>;

#[derive(Debug)]
pub struct SharedState {
    pub result: Result<SelectionOutput, SelectionError>,
    pub any_success: bool,
}

/// Struct for three arguments : target_for_match, match_range and target_feerate
///
/// Wrapped in a struct or else input for fn bnb takes too many arguments - 9/7
/// Leading to usage of stack instead of registers - https://users.rust-lang.org/t/avoiding-too-many-arguments-passing-to-a-function/103581
/// Fit in : 1 XMM register, 1 GPR
#[derive(Debug)]
pub struct MatchParameters {
    pub target_for_match: u64,
    pub match_range: u64,
    pub target_feerate: f32,
}
