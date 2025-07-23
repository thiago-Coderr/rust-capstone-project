use bitcoin::util::amount::Amount;
use bitcoin::{Address, Txid};
use bitcoincore_rpc::{Auth, Client, RpcApi};
use serde_json::json;
use std::fs::File;
use std::io::Write;
use std::thread::sleep;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let miner_rpc = Client::new(
        "http://127.0.0.1:18443",
        Auth::UserPass("user".to_string(), "password".to_string()),
    )?;
    let trader_rpc = Client::new(
        "http://127.0.0.1:18444",
        Auth::UserPass("user".to_string(), "password".to_string()),
    )?;

    let miner_address = miner_rpc.get_new_address(None, None)?;
    let trader_address = trader_rpc.get_new_address(None, None)?;

    // Send BTC from miner to trader
    let txid = miner_rpc.send_to_address(
        &trader_address,
        Amount::from_btc(20.0).unwrap(),
        None,
        None,
        None,
        None,
        None,
        None,
    )?;

    // Transaction is in mempool, confirm it
    miner_rpc.generate_to_address(1, &miner_address)?;

    // Wait a moment to ensure block is processed
    sleep(Duration::from_secs(1));

    // Fetch transaction info
    let tx_info = miner_rpc.get_transaction(&txid, Some(true))?;
    let tx = tx_info.details.first().unwrap();

    // Fetch raw transaction and decode to get output details
    let raw_tx = miner_rpc.get_raw_transaction(&txid, None)?;
    let decoded_tx = miner_rpc.decode_raw_transaction(&raw_tx)?;

    // Extract information
    let input_address = &tx.address;
    let input_amount = tx.amount.to_btc();

    let mut trader_output_address = String::new();
    let mut trader_output_amount = 0.0;
    let mut miner_change_address = String::new();
    let mut miner_change_amount = 0.0;

    for vout in &decoded_tx.vout {
        let address = vout
            .script_pub_key
            .addresses
            .as_ref()
            .unwrap()
            .first()
            .unwrap();
        if address == &trader_address.to_string() {
            trader_output_address = address.clone();
            trader_output_amount = vout.value;
        } else {
            miner_change_address = address.clone();
            miner_change_amount = vout.value;
        }
    }

    let fee = input_amount - (trader_output_amount + miner_change_amount);
    let block_hash = tx_info.info.blockhash.unwrap();
    let block = miner_rpc.get_block_info(&block_hash)?;
    let block_height = block.height;

    let mut file = File::create("../out.txt")?;
    writeln!(file, "Transaction ID (txid): {}", txid)?;
    writeln!(file, "Miner's Input Address: {}", input_address)?;
    writeln!(file, "Miner's Input Amount (in BTC): {:.8}", input_amount)?;
    writeln!(file, "Trader's Output Address: {}", trader_output_address)?;
    writeln!(
        file,
        "Trader's Output Amount (in BTC): {:.8}",
        trader_output_amount
    )?;
    writeln!(file, "Miner's Change Address: {}", miner_change_address)?;
    writeln!(
        file,
        "Miner's Change Amount (in BTC): {:.8}",
        miner_change_amount
    )?;
    writeln!(file, "Transaction Fees (in BTC): {:.8}", fee)?;
    writeln!(file, "Block height at which the transaction is confirmed: {}", block_height)?;
    writeln!(file, "Block hash at which the transaction is confirmed: {}", block_hash)?;

    Ok(())
}
