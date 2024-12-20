use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_coinselect::{
    select_coin_lowestlarger, CoinSelectionOpt, ExcessStrategy, OutputGroup, SelectionError,
    SelectionOutput,
};

fn benchmark_select_coin_lowestlarger(c: &mut Criterion) {
    let inputs = vec![
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
    ];

    let options = CoinSelectionOpt {
        target_value: 20000,
        target_feerate: 0.4,
        long_term_feerate: Some(0.4),
        min_absolute_fee: 0,
        base_weight: 10,
        drain_weight: 50,
        drain_cost: 10,
        cost_per_input: 20,
        cost_per_output: 10,
        min_drain_value: 500,
        excess_strategy: ExcessStrategy::ToDrain,
    };

    c.bench_function("select_coin_lowestlarger", |b| {
        b.iter(|| {
            let result: Result<SelectionOutput, SelectionError> =
                select_coin_lowestlarger(black_box(&inputs), black_box(options));
            let _ = black_box(result);
        })
    });
}

criterion_group!(benches, benchmark_select_coin_lowestlarger);
criterion_main!(benches);
