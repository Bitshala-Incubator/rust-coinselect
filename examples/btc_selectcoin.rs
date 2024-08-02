#![allow(unused)]
extern crate bitcoin;
#[macro_use]
extern crate serde;
extern crate serde_json;
use bitcoin::{
    address::NetworkUnchecked, Address, Amount, OutPoint, ScriptBuf, Sequence, TxIn, Witness,
};
use rust_coinselect::{
    select_coin, CoinSelectionOpt, ExcessStrategy, OutputGroup, SelectionError, SelectionOutput,
};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::str::FromStr;

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ListUnspentResultEntry {
    pub txid: bitcoin::Txid,
    pub vout: u32,
    pub address: Option<Address<NetworkUnchecked>>,
    pub label: Option<String>,
    pub redeem_script: Option<ScriptBuf>,
    pub witness_script: Option<ScriptBuf>,
    pub script_pub_key: ScriptBuf,
    #[serde(with = "bitcoin::amount::serde::as_btc")]
    pub amount: Amount,
    pub confirmations: u32,
    pub spendable: bool,
    pub solvable: bool,
    #[serde(rename = "desc")]
    pub descriptor: Option<String>,
    pub safe: bool,
}
#[derive(Debug, Clone)]
enum UTXOSpendInfo {
    SeedCoin {
        path: String,
        input_value: u64,
    },
    SwapCoin {
        multisig_redeemscript: ScriptBuf,
    },
    TimelockContract {
        swapcoin_multisig_redeemscript: ScriptBuf,
        input_value: u64,
    },
    HashlockContract {
        swapcoin_multisig_redeemscript: ScriptBuf,
        input_value: u64,
    },
    FidelityBondCoin {
        index: u32,
        input_value: u64,
    },
}
// Creating wrappers for implementing Display trait
#[derive(Debug)]
struct MySelectionOutput(SelectionOutput);
#[derive(Debug)]
struct MySelectionError(SelectionError);
fn create_coin_vector() -> Result<Vec<(ListUnspentResultEntry, UTXOSpendInfo)>, Box<dyn Error>> {
    let mut utxo = vec![ListUnspentResultEntry{
        txid: bitcoin::Txid::from_str("778c1c5010d131cc325ad425761d12e921fe17bc5397bd65b2c06dee2914d622").unwrap(),
        vout: 0,
        address: Some(Address::from_str("bcrt1qe7eru3nx4ngjvy8q7jsmlw8t93tk0flqgd354a").unwrap()),
        label: None,
        redeem_script: None,
        witness_script: Some(ScriptBuf::from_hex("02473044022012ae072ec02147df911cea2b12f63c40519151a32525a793dfff2828ae8b72c0022045dccec67916ff47e6a03c8b20511b7bc8d99c745fd9a669b4f35ffdba99fa38012103a0367f5808b41cffb7cb53913f24cf7cc564731815f7b9bf9459fc12ad4f96da").unwrap()),
        script_pub_key: ScriptBuf::from_hex("0014ed0bfc1766533f3aec5c16bfea47743f3c7b9bac").unwrap(),
        amount: Amount::from_btc(0.01)?,
        confirmations: 6,
        spendable: true,
        solvable: true,
        descriptor: Some("addr(bcrt1qmwef6jf68fcgsnxwgxecmnjjse6ccqt0vmfe6m)#cxppp4j9".to_string()),
        safe:true,
    }];
    utxo.push(ListUnspentResultEntry{
        txid: bitcoin::Txid::from_str("28379d9b08f25571b77eda051626e5678c1d26c72862728d07c4b88eb2d8a199").unwrap(),
        vout: 1,
        address: Some(Address::from_str("bcrt1q2cv50s90xfn87q3d62eh8d7epvxwjn2qc7hzc9").unwrap()),
        label: None,
        redeem_script: None,
        witness_script: Some(ScriptBuf::from_hex("0247304402200aa4e271eb95323bd39760c161fa61424e845d63ba3f2ad09038416782ec55ba022028c0f9bec25b5b02eeeb9bf50a21922d068602b87d7bd167e0915b102df8ecef0121036e5e401b53f0438b7c998d29e7f419ddf0a458248e609e2e95ffbffcfd4db2bf").unwrap()),
        script_pub_key: ScriptBuf::from_hex("0014ced4ec28769f97ef278ed712bb595b9a7689ce59").unwrap(),
        amount: Amount::from_btc(0.02)?,
        confirmations: 240,
        spendable: true,
        solvable: true,
        descriptor: Some("addr(bcrt1qe7eru3nx4ngjvy8q7jsmlw8t93tk0flqgd354a)#hu45l3fl".to_string()),
        safe:true,
    });
    utxo.push(ListUnspentResultEntry{
        txid: bitcoin::Txid::from_str("fb8b168d59c3cfb3ed1d76ea7be4c5e4ed2eef7f96a63632246e3825268421aa").unwrap(),
        vout: 0,
        address: Some(Address::from_str("bcrt1q2cv50s90xfn87q3d62eh8d7epvxwjn2qc7hzc9").unwrap()),
        label: None,
        redeem_script: None,
        witness_script: Some(ScriptBuf::from_hex("02483045022100a6ca6acfbe8461f266927357d68c06dfc60db31fbb0843dd1a67cba99e7f98e102207332a07923870f7a2c94b20f3aa5fe47243f149e78a3bbd70ff3f4f9eac00682012103673f43a91e45786a631e4fdcfde635401f3f045e937de6c69b5862326106112e").unwrap()),
        script_pub_key: ScriptBuf::from_hex("0014738d43e1913896daccedf6b4906e2165debd5705").unwrap(),
        amount: Amount::from_btc(0.05)?,
        confirmations: 20,
        spendable: true,
        solvable: true,
        descriptor: Some("addr(bcrt1q2cv50s90xfn87q3d62eh8d7epvxwjn2qc7hzc9)#kz8hz9t9".to_string()),
        safe:true,
    });
    utxo.push(ListUnspentResultEntry{
        txid: bitcoin::Txid::from_str("71d957c51cfb365fe526cf17378cb438cb8def5d3e08a653cc3534dedc181985").unwrap(),
        vout: 0,
        address: Some(Address::from_str("bcrt1qxm2f78ruvpcrugv0y330rwvhq6zsyrdherg26q").unwrap()),
        label: None,
        redeem_script: None,
        witness_script: Some(ScriptBuf::from_hex("02483045022100f21f4ee7a8561ac8d66617ad338ae45ecb488875f99b46ae33029ea130f2a6d202200478eacb38e2921dfd95fde197fdb6192287ccd15065731b903952dc512eaeb801210387eb83cb11dfb6233e9be8b6635c86144c8c04e9121c58f1af5b378b8ab6857d").unwrap()),
        script_pub_key: ScriptBuf::from_hex("0014ed0bfc1766533f3aec5c16bfea47743f3c7b9bac").unwrap(),
        amount: Amount::from_btc(0.1)?,
        confirmations: 17,
        spendable: true,
        solvable: true,
        descriptor: Some("addr(bcrt1qxm2f78ruvpcrugv0y330rwvhq6zsyrdherg26q)#3es6na43".to_string()),
        safe:true,
    });
    utxo.push(ListUnspentResultEntry{
        txid: bitcoin::Txid::from_str("c534488b7903cc9a73ff2d1452e4c5100a021238495171c643c78437a327c153").unwrap(),
        vout: 0,
        address: Some(Address::from_str("bcrt1qufnzxknsd6y42stxnnkven92dwqz8y6xndygqt").unwrap()),
        label: None,
        redeem_script: None,
        witness_script: Some(ScriptBuf::from_hex("02473044022069a0b11280a2059370b9e2632b8790fc2de738939ba4c7f713fc6c6f1d0b1441022009d265dd609fef98e6b21073bfd2696d32aed8b94297f736b1d20f266967aa130121037b00e6e487203e37475ce6c48cbd4b37eaebf69ef1e79e2221d1a2461ca06719").unwrap()),
        script_pub_key: ScriptBuf::from_hex("0014ed0bfc1766533f3aec5c16bfea47743f3c7b9bac").unwrap(),
        amount: Amount::from_btc(0.2)?,
        confirmations: 197,
        spendable: true,
        solvable: true,
        descriptor: Some("addr(bcrt1qufnzxknsd6y42stxnnkven92dwqz8y6xndygqt)#r9l2fknj".to_string()),
        safe:true,
    });
    // 0.01 BTC is a seedcoin
    let mut utxospendinfo: Vec<UTXOSpendInfo> = vec![UTXOSpendInfo::SeedCoin {
        path: "m/84'/0'/0/10".to_string(),
        input_value: 1,
    }];
    // 0.02 BTC is a seedcoin
    utxospendinfo.push(UTXOSpendInfo::SeedCoin {
        path: "m/84'/0'/0/14".to_string(),
        input_value: 1,
    });

    // 0.05 BTC is a seedcoin
    utxospendinfo.push(UTXOSpendInfo::SeedCoin {
        path: "m/84'/0'/0/143".to_string(),
        input_value: 1,
    });

    //0.1 BTC is a seedcoin
    utxospendinfo.push(UTXOSpendInfo::SeedCoin {
        path: "m/84'/0'/0/20".to_string(),
        input_value: 2,
    });

    //0.2 BTC is a seedcoin
    utxospendinfo.push(UTXOSpendInfo::SeedCoin {
        path: "m/84'/0'/0/7".to_string(),
        input_value: 1,
    });
    // let multisig_script_hex = "00473044022100d0ed946330182916da16a6149cd313a4b1a7b41591ee52fb3e79d64e36139d66021f6ccf173040ef24cb45c4db3e9c771c938a1ba2cf8d2404416f70886e360af401475121022afc20bf379bc96a2f4e9e63ffceb8652b2b6a097f63fbee6ecec2a49a48010e2103a767c7221e9f15f870f1ad9311f5ab937d79fcaeee15bb2c722bca515581b4c052ae";
    // match ScriptBuf::from_hex(multisig_script_hex){
    //     Ok(script) => utxospendinfo.push(UTXOSpendInfo::SwapCoin{multisig_redeemscript:script}),
    //     Err(e)=> println!("Error creating Redeem Script {}", e),
    //     }
    Ok(utxo.into_iter().zip(utxospendinfo).collect())
}
fn parse_listunspentresultentry_to_outputgroup(
    utxos: Vec<(ListUnspentResultEntry, UTXOSpendInfo)>,
) -> (Vec<OutputGroup>, CoinSelectionOpt) {
    // Creating TxIn struct to represent inputs
    let tx_inputs: Vec<TxIn> = utxos
        .iter()
        .map(|(entry, _)| TxIn {
            previous_output: OutPoint {
                txid: entry.txid,
                vout: entry.vout,
            },
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: match &entry.witness_script {
                Some(script) => {
                    let script_bytes = script.to_bytes();
                    Witness::from_slice(&[script_bytes])
                }
                None => Witness::default(),
            },
        })
        .collect();
    // Calculating the size of inputs
    let tx_inputs_size: Vec<usize> = tx_inputs.iter().map(|input| input.total_size()).collect();
    // Parsing ListUnspentResultEntry and tx_inputs_size to OutputGroup
    let inputs: Vec<OutputGroup> = utxos
        .iter()
        .zip(tx_inputs_size)
        .enumerate()
        .map(|(i, ((entry, _), size))| {
            OutputGroup {
                value: entry.amount.to_sat(),
                weight: size.try_into().unwrap_or(u32::MAX),
                input_count: 1, //Assuming each transaction has only one input
                is_segwit: entry.witness_script.is_some(), //If there exists a witness script, then the input is segwit
                creation_sequence: Some(i as u32), // Using the original index of the inputs in ListUnspentResultEntry vector to create squence
            }
        })
        .collect();
    let options = CoinSelectionOpt {
        target_value: 37900000,
        target_feerate: 0.4, // Simplified feerate
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
    (inputs, options)
}
impl fmt::Display for MySelectionOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Selected Inputs: {:?}, Waste: {:?}",
            self.0.selected_inputs, self.0.waste
        )
    }
}

impl fmt::Display for MySelectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            SelectionError::InsufficientFunds => write!(f, "Insufficient funds"),
            SelectionError::NoSolutionFound => write!(f, "No Solution Found"),
        }
    }
}
fn main() {
    let coins = match create_coin_vector() {
        Ok(coins) => coins,
        Err(e) => {
            println!("Error creating coin vector:{}", e);
            return;
        }
    };

    let (inputs, options) = parse_listunspentresultentry_to_outputgroup(coins);
    let selected_coin = select_coin(&inputs, options);
    match selected_coin {
        Ok(selection_output) => {
            let selection_output_wrapped = MySelectionOutput(selection_output);
            println!("Selected input indexes are: {:?}", selection_output_wrapped);
        }
        Err(selection_error) => {
            let selection_error_wrapped = MySelectionError(selection_error);
            println!("Selection Error: {:?}", selection_error_wrapped);
        }
    }
}
