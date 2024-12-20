use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_coinselect::{
    select_coin, CoinSelectionOpt, ExcessStrategy, OutputGroup, SelectionError, SelectionOutput,
};

fn benchmark_select_coin(c: &mut Criterion) {
    let inputs = [
        OutputGroup {
            value: 55000,
            weight: 500,
            input_count: 1,
            is_segwit: false,
            creation_sequence: None,
        },
        OutputGroup {
            value: 400,
            weight: 200,
            input_count: 1,
            is_segwit: false,
            creation_sequence: None,
        },
        OutputGroup {
            value: 40000,
            weight: 300,
            input_count: 1,
            is_segwit: false,
            creation_sequence: None,
        },
        OutputGroup {
            value: 25000,
            weight: 100,
            input_count: 1,
            is_segwit: false,
            creation_sequence: None,
        },
        OutputGroup {
            value: 35000,
            weight: 150,
            input_count: 1,
            is_segwit: false,
            creation_sequence: None,
        },
        OutputGroup {
            value: 600,
            weight: 250,
            input_count: 1,
            is_segwit: false,
            creation_sequence: None,
        },
        OutputGroup {
            value: 30000,
            weight: 120,
            input_count: 1,
            is_segwit: false,
            creation_sequence: None,
        },
        OutputGroup {
            value: 5000,
            weight: 50,
            input_count: 1,
            is_segwit: false,
            creation_sequence: None,
        },
    ];

    let options = CoinSelectionOpt {
        target_value: 5730,
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
    };

    c.bench_function("select_coin", |b| {
        b.iter(|| {
            let result: Result<SelectionOutput, SelectionError> =
                select_coin(black_box(&inputs), black_box(options));
            let _ = black_box(result);
        })
    });
}

criterion_group!(benches, benchmark_select_coin);
criterion_main!(benches);
