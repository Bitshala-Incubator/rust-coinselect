/*  An example showing how to use the rust bitcoin crate with the coinselect crate. The input and output details are stored in separate JSON files. The inputs and outputs are first read from the file and UTXOs are constructed using a combination of the inputs and outputs. The coin selection options are initiated. The UTXOs are then converted in OutputGroups. Finally the vector of OutputGroups and CoinSelectionOpt are used to call the coin selection method to perform the selection operation.
*/
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
    utils::{calculate_base_weight_btc, calculate_fee},
};
use std::str::FromStr;

fn log_utxos(utxos: &[OutputGroup]) {
    println!(
        "\n{:<15} | {:<15} | {:<15} | {:<20}",
        "Value (sats)", "Weight (bytes)", "Input Count", "Creation Sequence"
    );
    println!("{:-<71}", "");

    for utxo in utxos.iter() {
        println!(
            "{:<15} | {:<15} | {:<15} | {:<20}",
            utxo.value,
            utxo.weight,
            utxo.input_count,
            utxo.creation_sequence.unwrap_or(0)
        );
    }
    println!("{:-<71}", "");
}

fn main() {
    let target: u64 = 4_000_000;

    let inputs: Vec<TxIn> = vec![
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
    ];

    // Pre-calculate transaction weight using a dummy change output
    // Rationale:
    // 1. Transaction weight calculation requires both inputs and outputs
    // 2. We know the target output value (4M sats) but not the change amount yet
    // 3. Using 0 as placeholder for change output is valid because:
    //    - Output weight in Bitcoin depends on script size, not value
    //    - Value field is fixed 8 bytes regardless of amount
    //    - Only the script pubkey size affects weight
    let mut change_value = 0;
    const SCRIPT: &str = "a91409f6eed90e2ec7fed923b3d0b9d026efded6335c87";

    fn change_output(change_value: u64) -> TxOut {
        TxOut {
            value: Amount::from_sat(change_value),
            script_pubkey: ScriptBuf::from_hex(SCRIPT).unwrap(),
        }
    }
    let target_output = TxOut {
        value: Amount::from_sat(target),
        script_pubkey: ScriptBuf::from_hex(SCRIPT).unwrap(),
    };
    let outputs = vec![target_output.clone(), change_output(change_value)];

    fn create_tx(inputs: Vec<TxIn>, outputs: Vec<TxOut>) -> Transaction {
        Transaction {
            version: transaction::Version::TWO,
            lock_time: LockTime::ZERO,
            input: inputs,
            output: outputs,
        }
    }

    let output_weight_sum = outputs.iter().map(|output| output.weight().to_wu()).sum();

    // Create coin selection options
    let coin_selection_option = CoinSelectionOpt {
        target_value: target,
        target_feerate: 5.0,
        long_term_feerate: Some(10.0),
        min_absolute_fee: 4000,
        base_weight: calculate_base_weight_btc(output_weight_sum),
        change_weight: 34,
        change_cost: 8,
        avg_input_weight: inputs
            .iter()
            .map(|input| u64::from(input.segwit_weight()))
            .sum::<u64>()
            / inputs.len() as u64,
        avg_output_weight: output_weight_sum / 2,
        min_change_value: 100,
        excess_strategy: ExcessStrategy::ToChange,
    };

    // Mock values for each input
    let mock_input_values = vec![100_000, 500_000, 1_000_000, 3_000_000];

    // Create OutputGroups from each input
    let utxos: Vec<OutputGroup> = inputs
        .clone()
        .into_iter()
        .zip(mock_input_values)
        .map(|(input, value)| OutputGroup {
            // In practice, the details about the UTXO, used as input, is obtained from the UTXO set maintained by a node.
            value,
            weight: u64::from(input.segwit_weight()),
            input_count: 1,
            creation_sequence: None,
        })
        .collect();

    // Perform selection among the available UTXOs, create final transaction
    log_utxos(&utxos);
    match select_coin(&utxos, &coin_selection_option) {
        Ok(selection) => {
            println!("Selected utxo index and waste metrics are: {:?}", selection);
            let selected_utxo_aggregate: u64 = selection
                .selected_inputs
                .iter()
                .map(|&i| utxos[i].value)
                .sum();

            // Create a transaction for the purpose of calculating a default weight first
            let fee = calculate_fee(
                u64::from(create_tx(inputs.clone(), outputs).weight()),
                coin_selection_option.long_term_feerate.unwrap(),
            );

            change_value = selected_utxo_aggregate - (target + fee);
            println!(
                "Transaction Outputs created with target: {target} and change: {change_value}"
            );
            create_tx(inputs, vec![target_output, change_output(change_value)]);
        }
        Err(_) => println!("Coin Selection failed!"),
    }
}
