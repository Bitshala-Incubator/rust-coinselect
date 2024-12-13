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
    utils::calculate_base_weight_btc,
};
use serde_derive::Deserialize;
use std::fs;
use std::{path::Path, str::FromStr};

// A struct to read and store transaction inputs from the JSON file
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TxInJson {
    txid: String,
    vout: u32,
    script_sig: String,
    sequence: String,
    witness: Vec<String>,
}

// A struct to read and store transaction outputs from the JSON file
#[derive(Deserialize)]
struct TxOutJson {
    value: f64,
    script_pubkey: String,
}

fn read_json_file(file_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Checking if the given path exists
    if !Path::new(file_path).exists() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "File not found",
        )));
    }
    fs::read_to_string(file_path).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

fn json_to_txin(filedata: &str) -> Result<Vec<TxIn>, Box<dyn std::error::Error>> {
    // Parse transaction input data from JSON file into TxIn struct of the bitcoin crate
    let tx_in_json_vec: Vec<TxInJson> = serde_json::from_str(filedata)?;
    tx_in_json_vec
        .into_iter()
        .map(|tx_inp| {
            let txid = Txid::from_str(&tx_inp.txid)?;
            let script_signature = ScriptBuf::from_hex(&tx_inp.script_sig)?;
            let nsequence = Sequence::from_hex(&tx_inp.sequence)?;
            // Converting array of strings to slice of bytes
            let witness: Vec<&str> = tx_inp.witness.iter().map(|w| &w[..]).collect();
            // Converting from slice of bytes to Witness object
            let witnessdata = Witness::from_slice(&witness);
            Ok(TxIn {
                previous_output: OutPoint {
                    txid,
                    vout: tx_inp.vout,
                },
                script_sig: script_signature,
                sequence: nsequence,
                witness: witnessdata,
            })
        })
        .collect()
}

fn json_to_txout(filedata: &str) -> Result<Vec<TxOut>, Box<dyn std::error::Error>> {
    // Parse transaction output data from JSON file into TxOut struct of the bitcoin crate
    let tx_out_json_vec: Vec<TxOutJson> = serde_json::from_str(filedata)?;
    tx_out_json_vec
        .into_iter()
        .map(|tx_op| {
            let op_amount = Amount::from_btc(tx_op.value)?;
            let op_script_pubkey = ScriptBuf::from_hex(&tx_op.script_pubkey)?;
            Ok(TxOut {
                value: op_amount,
                script_pubkey: op_script_pubkey,
            })
        })
        .collect()
}

// The 'Transaction' struct of the bitcoin crate represents an UTXO. Here, the inputs (TxIn) and outputs (TxOut) are used to construct the 'Transaction' struct.
fn create_transaction(txinput: Vec<TxIn>, txoutput: Vec<TxOut>) -> Transaction {
    // Create a new transaction with the given vector of inputs and outputs. Assuming version = 2 and locktime = 0
    Transaction {
        version: transaction::Version::TWO,
        lock_time: LockTime::ZERO,
        input: txinput,
        output: txoutput,
    }
}

// UTXO (Transaction) is a combination of inputs and outputs. Here we pick inputs and outputs from the vector of TxIn and TxOut to construct the UTXO.
fn compose_transactions(inputs: Vec<TxIn>, outputs: Vec<TxOut>) -> Vec<Transaction> {
    // Generate combinations of inputs and outputs and create transactions using them
    vec![
        // UTXO 1 : Two inputs and one output
        create_transaction(
            inputs.iter().take(2).cloned().collect(),
            vec![outputs[2].clone()],
        ),
        // UTXO 2 : Three inputs and three outputs
        create_transaction(
            inputs.iter().skip(2).take(3).cloned().collect(),
            outputs.iter().skip(3).take(3).cloned().collect(),
        ),
        // UTXO 3: Five inputs and five outputs
        create_transaction(
            inputs.iter().take(5).cloned().collect(),
            outputs.iter().skip(1).take(5).cloned().collect(),
        ),
        // UTXO 4: One input and 7 outputs
        create_transaction(
            vec![inputs[5].clone()],
            outputs.iter().take(7).cloned().collect(),
        ),
        // UTXO 5: Two inputs and one output
        create_transaction(
            inputs.iter().skip(4).take(2).cloned().collect(),
            vec![outputs[6].clone()],
        ),
    ]
}

fn create_outputgroup(tx: Vec<Transaction>) -> Vec<OutputGroup> {
    // Create OutputGroup from transaction data
    tx.into_iter()
        .enumerate()
        .map(|(i, tx)| OutputGroup {
            value: tx.output.iter().map(|op| op.value.to_sat()).sum(),
            weight: tx.total_size() as u64,
            input_count: tx.input.len(),
            creation_sequence: Some(i as u32),
        })
        .collect()
}

fn create_select_options(output: &TxOut) -> Vec<CoinSelectionOpt> {
    let target_values = [50_000, 100_000, 200_000, 500_000, 1_000_000];
    let feerates = [1.0, 2.0, 3.0, 5.0, 10.0];
    // Use the weight() method from TxOut
    let base_weight = calculate_base_weight_btc(output.weight().to_wu());
    println!("Base weight: {}", base_weight);

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

fn perform_select_coin(utxos: Vec<OutputGroup>, coin_select_options_vec: Vec<CoinSelectionOpt>) {
    // Printing information about the UTXOs used for selection
    println!("\nThe total number of UTXOs available: {:?}", utxos.len());
    for (i, utxo) in utxos.iter().enumerate() {
        println!("\nUTXO #:{}", i);
        println!("Value:{} sats", utxo.value);
        println!("Weight:{} bytes", utxo.weight);
        println!("No. of Inputs: {}", utxo.input_count);
        println!(
            "Creation Sequence: {:?}",
            utxo.creation_sequence.unwrap_or(0)
        );
    }

    for coin_select_options in coin_select_options_vec.iter().take(5) {
        println!(
            "\nSelecting UTXOs to total: {:?} sats",
            coin_select_options.target_value
        );
        match select_coin(&utxos, coin_select_options) {
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
    // Read and parse inputs from JSON file
    let inputs = match read_json_file("examples/bitcoin_crate/txdata/txinp.json")
        .and_then(|filedata| json_to_txin(&filedata))
    {
        Ok(txin_vec) => txin_vec,
        Err(e) => {
            println!("Error reading or parsing inputs: {:?}", e);
            return;
        }
    };

    // Read and parse outputs from JSON file
    let outputs = match read_json_file("examples/bitcoin_crate/txdata/txop.json")
        .and_then(|filedata| json_to_txout(&filedata))
    {
        Ok(txout_vec) => txout_vec,
        Err(e) => {
            println!("Error reading or parsing outputs: {:?}", e);
            return;
        }
    };

    // Create new transactions using all possible combinations of inputs and outputs
    let transactions = compose_transactions(inputs.clone(), outputs.clone());
    // Create UTXOs of type OutputGroup to be passed to coin selection
    let utxos = create_outputgroup(transactions);
    // Create options for coin selection - modify to pass reference
    let coin_selection_options = create_select_options(&outputs[0]);

    // Perform coin selection
    perform_select_coin(utxos, coin_selection_options);
}
