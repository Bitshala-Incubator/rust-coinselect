/// Represents an input candidate for Coinselection, either as a single UTXO or a group of UTXOs.
///
/// A [`OutputGroup`] can be a single UTXO or a group that should be spent together.
/// For privacy reasons it might be a good choice to spend a group of UTXOs together.
/// In the UTXO model the output of a transaction is used as the input for the new transaction and hence the name [`OutputGroup`]
/// The library user must craft this structure correctly, as incorrect representation can lead to incorrect selection results.
#[derive(Debug, Clone)]
pub struct OutputGroup {
    /// Total value of the UTXO(s) that this [`WeightedValue`] represents.
    pub value: u64,
    /// Total weight of including these UTXO(s) in the transaction.
    ///
    /// The `txin` fields: `prevout`, `nSequence`, `scriptSigLen`, `scriptSig`, `scriptWitnessLen`,
    /// and `scriptWitness` should all be included.
    pub weight: u64,
    /// The total number of inputs
    pub input_count: usize,
    /// Specifies the relative creation sequence for this group, used only for FIFO selection.
    ///
    /// Set to `None` if FIFO selection is not required. Sequence numbers are arbitrary indices that denote the relative age of a UTXO group among a set of groups.
    /// To denote the oldest UTXO group, assign it a sequence number of `Some(0)`.
    pub creation_sequence: Option<u32>,
}

/// Options required to compute fees and waste metric.
#[derive(Debug, Clone)]
pub struct CoinSelectionOpt {
    /// The value we need to select.
    pub target_value: u64,

    /// The target feerate we should try and achieve in sats per weight unit.
    pub target_feerate: f32,

    /// The long term feerate affects how the [`WasteMetric`] is computed.
    /// If `target_feerate < long_term_feerate` then it's a good time to spend meaning less waste.
    pub long_term_feerate: Option<f32>,

    /// The minimum absolute fee. I.e., needed for RBF.
    pub min_absolute_fee: u64,

    /// Weights of data in transaction other than the list of inputs that would be selected.
    ///
    /// This includes weight of the header, total weight out outputs, weight of fields used
    /// to represent number number of inputs and number outputs, witness etc.,
    pub base_weight: u64,

    /// Additional weight if we include the change output.
    ///
    /// Used in weight metric computation.
    pub change_weight: u64,

    /// Weight of spending the change output in the future.
    pub change_cost: u64,

    /// Estimate of average weight of an input.
    pub avg_input_weight: u64,

    /// Estimate of average weight of an output.
    pub avg_output_weight: u64,

    /// Minimum value allowed for a change output to avoid dusts.
    pub min_change_value: u64,

    /// Strategy to use the excess value other than fee and target
    pub excess_strategy: ExcessStrategy,
}

/// Strategy to decide what to do with the excess amount.
#[derive(Clone, Debug, PartialEq, Eq)]
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

/// Measures the efficiency of input selection in satoshis, helping evaluate algorithms based on current and long-term fee rates
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

/// EffectiveValue type alias
pub type EffectiveValue = u64;

/// Weight type alias
pub type Weight = u64;
