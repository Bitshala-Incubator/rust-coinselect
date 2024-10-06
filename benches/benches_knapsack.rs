use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_coinselect::{
    select_coin_knapsack, CoinSelectionOpt, ExcessStrategy, OutputGroup, SelectionError,
    SelectionOutput,
};

const CENT: u64 = 1_000_000;

fn benchmark_select_coin_knapsack(c: &mut Criterion) {
    let inputs = {
        const VALUE: [u64; 5] = [
            6 * CENT,
            7 * CENT,
            8 * CENT,
            20 * CENT,
            30 * CENT,
        ];
        const WEIGHTS: [u32; 5] = [100, 200, 100, 10, 5];
        const TARGET_FEERATE: f32 = 0.77;

        VALUE.iter()
            .zip(WEIGHTS.iter())
            .map(|(&i, &j)| {
                OutputGroup {
                    value: i.saturating_add((j as f32 * TARGET_FEERATE).ceil() as u64),
                    weight: j,
                    input_count: 1,
                    is_segwit: false,
                    creation_sequence: None,
                }
            })
            .collect::<Vec<OutputGroup>>()
    };

    let options = {
        const MIN_DRAIN_VALUE: u64 = 500;
        const BASE_WEIGHT: u32 = 10;
        const TARGET_FEERATE: f32 = 0.56;
        const ADJUSTED_TARGET: u64 = 37 * CENT;

        CoinSelectionOpt {
            target_value: ADJUSTED_TARGET - MIN_DRAIN_VALUE - (BASE_WEIGHT as f32 * TARGET_FEERATE).ceil() as u64,
            target_feerate: TARGET_FEERATE,
            long_term_feerate: Some(0.4),
            min_absolute_fee: 0,
            base_weight: BASE_WEIGHT,
            drain_weight: 50,
            drain_cost: 10,
            cost_per_input: 20,
            cost_per_output: 10,
            min_drain_value: MIN_DRAIN_VALUE,
            excess_strategy: ExcessStrategy::ToDrain,
        }
    };

    c.bench_function("select_coin_knapsack", |b| {
        b.iter(|| {
            let result: Result<SelectionOutput, SelectionError> =
                select_coin_knapsack(black_box(&inputs), black_box(options));
            let _ = black_box(result);
        })
    });
}

criterion_group!(benches, benchmark_select_coin_knapsack);
criterion_main!(benches);
