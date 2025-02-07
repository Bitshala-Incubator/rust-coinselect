use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_coinselect::{
    algorithms::srd::select_coin_srd,
    types::{CoinSelectionOpt, ExcessStrategy, OutputGroup, SelectionError, SelectionOutput},
};

fn benchmark_select_coin_srd(c: &mut Criterion) {
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
        target_feerate: 0.5,
        long_term_feerate: None,
        min_absolute_fee: 0,
        base_weight: 10,
        change_weight: 50,
        change_cost: 10,
        avg_input_weight: 20,
        avg_output_weight: 10,
        min_change_value: 500,
        excess_strategy: ExcessStrategy::ToChange,
    };

    let mut final_result: Option<Result<SelectionOutput, SelectionError>> = None;

    c.bench_function("select_coin_srd", |b| {
        b.iter(|| {
            final_result = Some(select_coin_srd(black_box(&inputs), black_box(&options)));
            black_box(&final_result);
        })
    });

    if let Some(result) = &final_result {
        match result {
            Ok(selection) => println!("SelectionOutput: {:?}", selection),
            Err(e) => println!("Error: {:?}", e),
        }
    }
}

criterion_group!(benches, benchmark_select_coin_srd);
criterion_main!(benches);
