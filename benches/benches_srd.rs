use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_coinselect::{
    select_coin_srd, CoinSelectionOpt, ExcessStrategy, OutputGroup, SelectionError, SelectionOutput,
};

fn benchmark_select_coin_srd(c: &mut Criterion) {
    let inputs = vec![
        OutputGroup {
            value: 1000,
            weight: 100,
            input_count: 1,
            is_segwit: false,
            creation_sequence: None,
        },
        OutputGroup {
            value: 2000,
            weight: 200,
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
    ];

    let options = CoinSelectionOpt {
        target_value: 2500,
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

    c.bench_function("select_coin_srd", |b| {
        b.iter(|| {
            let result: Result<SelectionOutput, SelectionError> =
                select_coin_srd(black_box(&inputs), black_box(options));
            let _ = black_box(result);
        })
    });
}

criterion_group!(benches, benchmark_select_coin_srd);
criterion_main!(benches);
