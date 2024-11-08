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
use rand::Rng;
use rust_coinselect::{
    selectcoin::select_coin,
    types::{CoinSelectionOpt, ExcessStrategy, OutputGroup},
};
use serde_derive::Deserialize;
use std::fs;
use std::{collections::HashSet, path::Path, str::FromStr};

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
    // Cheking if the given path exists
    if !Path::new(file_path).exists() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "File not found",
        )));
    }
    match fs::read_to_string(file_path) {
        Ok(file_content) => Ok(file_content),
        Err(e) => Err(Box::new(e)),
    }
}
fn json_to_txin(filedata: &str) -> Result<Vec<TxIn>, Box<dyn std::error::Error>> {
    // Parse transaction output data from JSON file into TxIn struct of the bitcoin crate
    let tx_in_json_vec: Vec<TxInJson> = serde_json::from_str(filedata)?;
    let mut tx_in_vec: Vec<TxIn> = Vec::new();
    for tx_inp in tx_in_json_vec {
        let txid = Txid::from_str(&tx_inp.txid)?;
        let vout = tx_inp.vout;
        let script_signature = ScriptBuf::from_hex(&tx_inp.script_sig)?;
        let nsequence = Sequence::from_hex(&tx_inp.sequence)?;
        // Converting array of strings to slice of bytes
        let witness: Vec<&str> = tx_inp.witness.iter().map(|w| &w[..]).collect();
        // Converting from slice of bytes to Witness object
        let witnessdata = Witness::from_slice(&witness);
        tx_in_vec.push(TxIn {
            previous_output: OutPoint { txid, vout },
            script_sig: script_signature,
            sequence: nsequence,
            witness: witnessdata,
        });
    }

    Ok(tx_in_vec)
}

fn json_to_txout(filedata: &str) -> Result<Vec<TxOut>, Box<dyn std::error::Error>> {
    // Parse transaction output data from JSON file into TxOut struct of the bitcoin crate
    let tx_out_json_vec: Vec<TxOutJson> = serde_json::from_str(filedata)?;
    let mut tx_out_vec: Vec<TxOut> = Vec::new();
    for tx_op in tx_out_json_vec {
        let op_amount = Amount::from_btc(tx_op.value)?;
        let op_script_pubkey = ScriptBuf::from_hex(&tx_op.script_pubkey)?;
        tx_out_vec.push(TxOut {
            value: op_amount,
            script_pubkey: op_script_pubkey,
        });
    }

    Ok(tx_out_vec)
}
// The 'Transaction' struct of the bitcoin crate represents an UTXO. Here, the inputs (TxIn) and outputs (TxOut) are used to construct the 'Transaction' struct.
fn create_transaction(
    txinput: Vec<TxIn>,
    txoutput: Vec<TxOut>,
) -> Result<Transaction, Box<dyn std::error::Error>> {
    // Create a new transaction with the given vector of inputs and outputs. Assuming version = 2 and locktime = 0
    Ok(Transaction {
        version: transaction::Version::TWO,
        lock_time: LockTime::ZERO,
        input: txinput,
        output: txoutput,
    })
}
// UTXO (Transaction) is a combination of inputs and outputs. Here we pick inputs and outputs from the vector of TxIn and TxOut to construct the UTXO.
fn compose_transaction(
    inputs: Vec<TxIn>,
    outputs: Vec<TxOut>,
) -> Result<Vec<Transaction>, Box<dyn std::error::Error>> {
    // Generate combinations of inputs and outputs and create transactions using them
    let mut transactions_vec: Vec<Transaction> = Vec::new();

    // UTXO 1 : Two inputs and one output
    let selected_inputs: Vec<TxIn> = inputs.iter().take(2).cloned().collect();
    let selected_outputs: Vec<TxOut> = vec![outputs[2].clone()];
    let new_transaction: Transaction = create_transaction(selected_inputs, selected_outputs)?;
    transactions_vec.push(new_transaction);

    // UTXO 2 : Three inputs and three outputs
    let selected_inputs: Vec<TxIn> = inputs.iter().skip(2).take(3).cloned().collect();
    let selected_outputs: Vec<TxOut> = outputs.iter().skip(3).take(3).cloned().collect();
    let new_transaction: Transaction = create_transaction(selected_inputs, selected_outputs)?;
    transactions_vec.push(new_transaction);

    // UTXO 3: Five inputs and five outputs
    let selected_inputs: Vec<TxIn> = inputs.iter().take(5).cloned().collect();
    let selected_outputs: Vec<TxOut> = outputs.iter().skip(1).take(5).cloned().collect();
    let new_transaction: Transaction = create_transaction(selected_inputs, selected_outputs)?;
    transactions_vec.push(new_transaction);
    // UTXO 4: One input and 7 ouputs
    let selected_inputs: Vec<TxIn> = vec![inputs[5].clone()];
    let selected_outputs: Vec<TxOut> = outputs.iter().take(7).cloned().collect();
    let new_transaction: Transaction = create_transaction(selected_inputs, selected_outputs)?;
    transactions_vec.push(new_transaction);
    // UTXO 5: Two inputs and one output
    let selected_inputs: Vec<TxIn> = inputs.iter().skip(4).take(2).cloned().collect();
    let selected_outputs: Vec<TxOut> = vec![outputs[6].clone()];
    let new_transaction: Transaction = create_transaction(selected_inputs, selected_outputs)?;
    transactions_vec.push(new_transaction);
    Ok(transactions_vec)
}

fn create_outputgroup(
    tx: Vec<Transaction>,
) -> Result<Vec<OutputGroup>, Box<dyn std::error::Error>> {
    // Create OutputGroup from transaction data
    let mut rng = rand::thread_rng();
    let mut output_group_vec: Vec<OutputGroup> = Vec::new();
    let total_transactions = tx.len();
    let mut unique_numbers: HashSet<u32> = HashSet::new();
    for tx in tx {
        let mut creation_sequence: u32;
        loop {
            creation_sequence = rng.gen_range(0..total_transactions as u32);
            if unique_numbers.insert(creation_sequence) {
                break;
            }
        }
        output_group_vec.push(OutputGroup {
            value: tx.output.iter().map(|op| op.value.to_sat()).sum(),
            weight: tx.total_size() as u32,
            input_count: tx.input.len(),
            creation_sequence: Some(creation_sequence),
        })
    }

    Ok(output_group_vec)
}

fn create_select_options() -> Result<Vec<CoinSelectionOpt>, Box<dyn std::error::Error>> {
    let mut rng = rand::thread_rng();
    let mut coin_select_options_vec: Vec<CoinSelectionOpt> = Vec::new();
    // Creating 5 different options for coin selection
    for _ in 0..5 {
        // Random selection of Excess Strategy
        let excess_strategy = match rng.gen_range(0..3) {
            0 => ExcessStrategy::ToChange,
            1 => ExcessStrategy::ToFee,
            2 => ExcessStrategy::ToRecipient,
            _ => unreachable!(),
        };
        coin_select_options_vec.push(CoinSelectionOpt {
            target_value: rng.gen_range(40000..5000000000i64) as u64,
            target_feerate: rng.gen_range(1.0..5.0) as f32,
            long_term_feerate: Some(rng.gen_range(1..10) as f32),
            min_absolute_fee: rng.gen_range(1..20) as u64,
            base_weight: rng.gen_range(1..30) as u32,
            change_weight: rng.gen_range(5..30) as u32,
            change_cost: rng.gen_range(1..20) as u64,
            cost_per_input: rng.gen_range(1..10) as u64,
            cost_per_output: rng.gen_range(1..10) as u64,
            min_change_value: rng.gen_range(100..1000) as u64,
            excess_strategy,
        })
    }
    Ok(coin_select_options_vec)
}

fn perform_select_coin(utxos: Vec<OutputGroup>, coin_select_options_vec: Vec<CoinSelectionOpt>) {
    // Printing information about the UTXOs used for slection
    println!("\nThe total numner of UTXOs available: {:?}", utxos.len());
    for (i, utxo) in utxos.iter().enumerate() {
        println!("\nUTXO #:{}", i);
        println!("\nValue:{} sats", utxo.value);
        println!("Weight:{} bytes", utxo.weight);
        println!("No. of Inputs: {}", utxo.input_count);
        println!(
            "Creation Sequence: {:?}",
            utxo.creation_sequence.unwrap_or(0)
        );
    }

    for (_, coin_select_options) in coin_select_options_vec.iter().enumerate().take(5) {
        println!(
            "\nSelecting UTXOs to total: {:?} sats",
            coin_select_options.target_value
        );
        match select_coin(&utxos, *coin_select_options) {
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
    //Read and parse inputs from JSON file
    let inputs = match read_json_file("examples/bitcoin_crate/txdata/txinp.json") {
        Ok(filedata) => match json_to_txin(&filedata) {
            Ok(txin_vec) => txin_vec,
            Err(e) => {
                println!("Error parsing json {:?}", e);
                return;
            }
        },
        Err(e) => {
            println!("Error reading file {:?}", e);
            return;
        }
    };
    // Read and parse outputs from JSON file
    let outputs = match read_json_file("examples/bitcoin_crate/txdata/txop.json") {
        Ok(filedata) => match json_to_txout(&filedata) {
            Ok(txout_vec) => txout_vec,
            Err(e) => {
                println!("Error parsing json {:?}", e);
                return;
            }
        },
        Err(e) => {
            println!("Error reading file {:?}", e);
            return;
        }
    };
    // Create a new transactions using all possible combinations of inputs and outputs
    let transactions = match compose_transaction(inputs, outputs) {
        Ok(transactions_vec) => transactions_vec,
        Err(e) => {
            println!("Error creating transactions {:?}", e);
            return;
        }
    };
    // Create UTXOs of type OutPutGroup to be passed to coinselection
    let utxos = match create_outputgroup(transactions) {
        Ok(output_group_vec) => output_group_vec,
        Err(e) => {
            println!("Error creating output group {:?}", e);
            return;
        }
    };
    // Create options for coin selection
    let coin_selection_options = match create_select_options() {
        Ok(coin_select_options_vec) => coin_select_options_vec,
        Err(e) => {
            println!("Error creating coin selection options {:?}", e);
            return;
        }
    };
    // Performing coin selection
    perform_select_coin(utxos, coin_selection_options)
}
