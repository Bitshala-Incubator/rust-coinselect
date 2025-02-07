use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_coinselect::{
    algorithms::lowestlarger::select_coin_lowestlarger,
    types::{CoinSelectionOpt, ExcessStrategy, OutputGroup, SelectionError, SelectionOutput},
};

fn benchmark_select_coin_lowestlarger(c: &mut Criterion) {
    let inputs = vec![
        OutputGroup {
            value: 100,
            weight: 100,
            input_count: 1,
            creation_sequence: None,
        },
        OutputGroup {
            value: 1500,
            weight: 200,
            input_count: 1,
            creation_sequence: None,
        },
        OutputGroup {
            value: 3400,
            weight: 300,
            input_count: 1,
            creation_sequence: None,
        },
        OutputGroup {
            value: 2200,
            weight: 150,
            input_count: 1,
            creation_sequence: None,
        },
        OutputGroup {
            value: 1190,
            weight: 200,
            input_count: 1,
            creation_sequence: None,
        },
        OutputGroup {
            value: 3300,
            weight: 100,
            input_count: 1,
            creation_sequence: None,
        },
        OutputGroup {
            value: 1000,
            weight: 190,
            input_count: 1,
            creation_sequence: None,
        },
        OutputGroup {
            value: 2000,
            weight: 210,
            input_count: 1,
            creation_sequence: None,
        },
        OutputGroup {
            value: 3000,
            weight: 300,
            input_count: 1,
            creation_sequence: None,
        },
        OutputGroup {
            value: 2250,
            weight: 250,
            input_count: 1,
            creation_sequence: None,
        },
        OutputGroup {
            value: 190,
            weight: 220,
            input_count: 1,
            creation_sequence: None,
        },
        OutputGroup {
            value: 1750,
            weight: 170,
            input_count: 1,
            creation_sequence: None,
        },
    ];

    let options = CoinSelectionOpt {
        target_value: 20000,
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

    let mut final_result: Option<Result<SelectionOutput, SelectionError>> = None;

    c.bench_function("select_coin_lowestlarger", |b| {
        b.iter(|| {
            final_result = Some(select_coin_lowestlarger(
                black_box(&inputs),
                black_box(&options),
            ));
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

criterion_group!(benches, benchmark_select_coin_lowestlarger);
criterion_main!(benches);
