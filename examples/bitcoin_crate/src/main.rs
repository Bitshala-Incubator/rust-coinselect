extern crate bitcoin;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;

use bitcoin::{
    absolute::LockTime, transaction, Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn,
    TxOut, Txid, Witness,
};
use rust_coinselect::{
    selectcoin::select_coin,
    types::{CoinSelectionOpt, ExcessStrategy, OutputGroup},
    utils::calculate_base_weight_btc,
};
use std::str::FromStr;

fn create_txouts() -> Vec<TxOut> {
    vec![
        TxOut {
            value: Amount::from_btc(0.2).unwrap(),
            script_pubkey: ScriptBuf::from_hex("a91409f6eed90e2ec7fed923b3d0b9d026efded6335c87")
                .unwrap(),
        },
        TxOut {
            value: Amount::from_btc(23.0).unwrap(),
            script_pubkey: ScriptBuf::from_hex("00142fffa9a09bb7fa7dced44834d77ee81c49c5f0cc")
                .unwrap(),
        },
    ]
}

fn create_txins() -> Vec<TxIn> {
    vec![
        TxIn {
            previous_output: OutPoint {
                txid: Txid::from_str("e9269b40306f10e2636414e514366474d5736256844882ffefd794f688b648b0").unwrap(),
                vout: 0,
            },
            script_sig: ScriptBuf::from_hex("47304402200552d42d255d9814bda2e0fbf28be6321a80615e633afb21d18abf4ad0bc721602202530e3f8e4bbf184eecfd95c615cb22ad06860fd75ce3aa423d0fea894a80332812103739ea9368ff2b1fdc4db2f160191e980190367501cc2b0c93a566fababd01064").unwrap(),
            sequence: Sequence::from_hex("0xffffffff").unwrap(),
            witness: Witness::default()
        },
        TxIn {
            previous_output: OutPoint {
                txid: Txid::from_str("e1926a751f36c25541263ca4b621f1e3376e15117c170e60ba82b113bda3cba6").unwrap(),
                vout: 0,
            },
            script_sig: ScriptBuf::from_hex("47304402203743aeedace4ec68892c825ed674234b7c834268304aaaf76958d2da5670a9b702207b01a50a5c047023ac5a5b90367b1bd862ac6a4914b023a7c5463b3f8e3690710141047146f0e0fcb3139947cf0beb870fe251930ca10d4545793d31033e801b5219abf56c11a3cf3406ca590e4c14b0dab749d20862b3adc4709153c280c2a78be10c").unwrap(),
            sequence: Sequence::from_hex("0xffffffff").unwrap(),
            witness: Witness::default()
        },
        TxIn {
            previous_output: OutPoint {
                txid: Txid::from_str("0c20171f1558eb6c032a5916e8ce0b446fef6f7b102b2aabcfc83a8367b0f53b").unwrap(),
                vout: 2,
            },
            script_sig: ScriptBuf::default(),
            sequence: Sequence::from_hex("0xfffffffe").unwrap(),
            witness: Witness::from_slice(&[
                "30440220079287854aed2500913921a7b9698cefc17fc68fb02728d2e86c52173b6af6ba0220669efe80b73204efcda6c3a375da5fd059977e6c86bad977b15c562219cc5dd601",
                "032d49f270684da5a422df37bf9818aa178ad89a975643c4928444aed78a7dd341"
            ]),
        },
        TxIn {
            previous_output: OutPoint {
                txid: Txid::from_str("f2ccf54fa95a13b092d4f90c9cb215d11c858fd2964a45f3f997b3d4815e24e4").unwrap(),
                vout: 1,
            },
            script_sig: ScriptBuf::default(),
            sequence: Sequence::from_hex("0xfffffffd").unwrap(),
            witness: Witness::from_slice(&[
                "30450221008c10e822049f4e67388cd9dc825d1bf14c7f8b73801bc025e7534b33319d510f02205c214beb89a3b3e0837b6cc717dbda4f8707076050197b870cffec7594040c8801",
                "02f2a0bdce72551e68dfbebf81d770924836cbb256a63e9987ea869eabac4859c1"
            ]),
        }
    ]
}

fn create_transaction(txinput: Vec<TxIn>, txoutput: Vec<TxOut>) -> Transaction {
    Transaction {
        version: transaction::Version::TWO,
        lock_time: LockTime::ZERO,
        input: txinput,
        output: txoutput,
    }
}

fn create_outputgroup(tx: Transaction) -> OutputGroup {
    OutputGroup {
        value: tx.output.iter().map(|op| op.value.to_sat()).sum(),
        weight: tx.total_size() as u64,
        input_count: tx.input.len(),
        creation_sequence: None,
    }
}

fn create_select_options(output_weight: u64) -> Vec<CoinSelectionOpt> {
    let target_values = [50_000, 100_000, 200_000, 500_000, 1_000_000];
    let feerates = [1.0, 2.0, 3.0, 5.0, 10.0];
    let base_weight = calculate_base_weight_btc(output_weight);

    (0..5)
        .map(|i| {
            let excess_strategy = match i % 3 {
                0 => ExcessStrategy::ToChange,
                1 => ExcessStrategy::ToFee,
                2 => ExcessStrategy::ToRecipient,
                _ => unreachable!(),
            };

            CoinSelectionOpt {
                target_value: target_values[i],
                target_feerate: feerates[i],
                long_term_feerate: Some(10.0),
                min_absolute_fee: 1000 * (i + 1) as u64,
                base_weight,
                change_weight: 34,
                change_cost: 5 + i as u64,
                avg_input_weight: 148,
                avg_output_weight: 34,
                min_change_value: 100,
                excess_strategy,
            }
        })
        .collect()
}

fn perform_select_coin(utxo: OutputGroup, coin_select_options_vec: Vec<CoinSelectionOpt>) {
    println!("Value:{} sats", utxo.value);
    println!("Weight:{} bytes", utxo.weight);
    println!("No. of Inputs: {}", utxo.input_count);
    println!(
        "Creation Sequence: {:?}",
        utxo.creation_sequence.unwrap_or(0)
    );

    for coin_select_options in coin_select_options_vec.iter().take(5) {
        println!(
            "\nSelecting UTXOs to total: {:?} sats",
            coin_select_options.target_value
        );
        match select_coin(&[utxo.clone()], coin_select_options) {
            Ok(selectionoutput) => {
                println!(
                    "Selected utxo index and waste metrics are: {:?}",
                    selectionoutput
                );
            }
            Err(e) => {
                println!("Error performing coin selection: {:?}", e);
            }
        }
    }
}

fn main() {
    // Create inputs and outputs manually
    let inputs = create_txins();
    let outputs = create_txouts();

    // Create a new transaction using the inputs and outputs
    let transaction = create_transaction(inputs.clone(), outputs.clone());

    // Create UTXOs of type OutputGroup to be passed to coin selection
    let utxo = create_outputgroup(transaction);

    // Create options for coin selection
    let coin_selection_options = create_select_options(outputs[0].weight().to_wu());

    // Perform coin selection
    perform_select_coin(utxo, coin_selection_options);
}
