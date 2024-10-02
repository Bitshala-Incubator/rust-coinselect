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

/// The result of selection algorithm
#[derive(Debug)]
pub struct SelectionOutput {
    /// The selected input indices, refers to the indices of the inputs Slice Reference
    pub selected_inputs: Vec<usize>,
    /// The waste amount, for the above inputs
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
    pub(crate) result: Result<SelectionOutput, SelectionError>,
    pub(crate) any_success: bool,
}

/// Struct for three arguments : target_for_match, match_range and target_feerate
///
/// Wrapped in a struct or else input for fn bnb takes too many arguments - 9/7
/// Leading to usage of stack instead of registers - https://users.rust-lang.org/t/avoiding-too-many-arguments-passing-to-a-function/103581
/// Fit in : 1 XMM register, 1 GPR
#[derive(Debug)]
pub struct MatchParameters {
    pub(crate) target_for_match: u64,
    pub(crate) match_range: u64,
    pub(crate) target_feerate: f32,
}
