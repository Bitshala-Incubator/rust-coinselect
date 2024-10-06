use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_coinselect::{
    select_coin_knapsack, CoinSelectionOpt, ExcessStrategy, OutputGroup, SelectionError,
    SelectionOutput,
};

const CENT: f64 = 1000000.0;

fn knapsack_setup_output_groups(
    value: Vec<u64>,
    weights: Vec<u32>,
    target_feerate: f32,
) -> Vec<OutputGroup> {
    let mut inputs: Vec<OutputGroup> = Vec::new();
    for (i, j) in value.into_iter().zip(weights.into_iter()) {
        let k = i.saturating_add((j as f32 * target_feerate).ceil() as u64);
        inputs.push(OutputGroup {
            value: k,
            weight: j,
            input_count: 1,
            is_segwit: false,
            creation_sequence: None,
        })
    }
    inputs
}

fn benchmark_select_coin_knapsack(c: &mut Criterion) {
    let inputs = knapsack_setup_output_groups(
        vec![
            (6.0 * CENT).round() as u64,
            (7.0 * CENT).round() as u64,
            (8.0 * CENT).round() as u64,
            (20.0 * CENT).round() as u64,
            (30.0 * CENT).round() as u64,
        ],
        vec![100, 200, 100, 10, 5],
        0.77,
    );

    let options = {
        let min_drain_value = 500;
        let base_weight = 10;
        let target_feerate = 0.56;
        let adjusted_target = (37.0 * CENT).round() as u64;
        let target_value =
            adjusted_target - min_drain_value - (base_weight as f32 * target_feerate).ceil() as u64;
        CoinSelectionOpt {
            target_value,
            target_feerate,
            long_term_feerate: Some(0.4),
            min_absolute_fee: 0,
            base_weight,
            drain_weight: 50,
            drain_cost: 10,
            cost_per_input: 20,
            cost_per_output: 10,
            min_drain_value,
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
