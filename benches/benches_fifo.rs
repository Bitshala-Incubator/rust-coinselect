use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_coinselect::{
    algorithms::fifo::select_coin_fifo,
    types::{CoinSelectionOpt, ExcessStrategy, OutputGroup, SelectionError, SelectionOutput},
};

fn benchmark_select_coin_fifo(c: &mut Criterion) {
    let inputs = vec![
        OutputGroup {
            value: 1000,
            weight: 100,
            input_count: 1,
            creation_sequence: None,
        },
        OutputGroup {
            value: 2000,
            weight: 200,
            input_count: 1,
            creation_sequence: None,
        },
        OutputGroup {
            value: 3000,
            weight: 300,
            input_count: 1,
            creation_sequence: None,
        },
    ];

    let options = CoinSelectionOpt {
        target_value: 2500,
        target_feerate: 0.4,
        long_term_feerate: Some(0.4),
        min_absolute_fee: 0,
        base_weight: 10,
        change_weight: 50,
        change_cost: 10,
        avg_input_weight: 20,
        avg_output_weight: 10,
        min_change_value: 500,
        excess_strategy: ExcessStrategy::ToChange,
    };

    c.bench_function("select_coin_fifo", |b| {
        b.iter(|| {
            let result: Result<SelectionOutput, SelectionError> =
                select_coin_fifo(black_box(&inputs), black_box(&options));
            let _ = black_box(result);
        })
    });
}

criterion_group!(benches, benchmark_select_coin_fifo);
criterion_main!(benches);
