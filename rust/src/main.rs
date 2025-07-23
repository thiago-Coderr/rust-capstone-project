use bitcoincore_rpc::{bitcoin::Amount, bitcoin::Address, bitcoin::Txid, Auth, Client, RpcApi};
use serde_json::json;
use std::fs::File;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let miner_rpc = Client::new(
        "http://127.0.0.1:18443",
        Auth::UserPass("user".to_string(), "password".to_string()),
    )?;
    let trader_rpc = Client::new(
        "http://127.0.0.1:18444",
        Auth::UserPass("user".to_string(), "password".to_string()),
    )?;

    let trader_address = trader_rpc.get_new_address(None, None)?;
    let trader_address = trader_address.require_network(bitcoincore_rpc::bitcoin::Network::Regtest)?;

    let miner_address = miner_rpc.get_new_address(None, None)?;
    let miner_address = miner_address.require_network(bitcoincore_rpc::bitcoin::Network::Regtest)?;

    // Generate block to fund the miner
    miner_rpc.generate_to_address(101, &miner_address)?;

    // Send BTC to the trader
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

    let tx = miner_rpc.get_transaction(&txid, None)?;
    let decoded_tx = miner_rpc.decode_raw_transaction(&tx.hex, None)?;

    let mut trader_output_amount = 0.0;
    let mut miner_change_amount = 0.0;

    for vout in &decoded_tx.vout {
        let address = vout
            .script_pub_key
            .addresses
            .as_ref()
            .unwrap()
            .first()
            .unwrap();

        if address.to_string() == trader_address.to_string() {
            trader_output_amount = vout.value.to_btc();
        } else if address.to_string() == miner_address.to_string() {
            miner_change_amount = vout.value.to_btc();
        }
    }

    let mut input_address = None;
    if let Some(vin) = decoded_tx.vin.first() {
        let prev_tx = miner_rpc.get_raw_transaction(&vin.txid, None)?;
        let decoded_prev_tx = miner_rpc.decode_raw_transaction(&prev_tx, None)?;
        if let Some(prev_vout) = decoded_prev_tx.vout.get(vin.vout as usize) {
            input_address = prev_vout
                .script_pub_key
                .addresses
                .as_ref()
                .unwrap()
                .first()
                .cloned();
        }
    }

    let fee = tx.details.iter().map(|d| d.fee.unwrap_or(Amount::from_btc(0.0).unwrap())).sum::<Amount>().to_btc();

    let block_hash = tx.info.blockhash.unwrap();
    let block = miner_rpc.get_block_info(&block_hash)?;
    let block_height = block.height;

    let mut file = File::create("../out.txt")?;
    writeln!(file, "Transaction ID: {}", txid)?;
    writeln!(file, "Miner's Input Address: {:?}", input_address)?;
    writeln!(file, "Trader's Receiving Address: {}", trader_address)?;
    writeln!(file, "Trader's Received Amount (in BTC): {:.8}", trader_output_amount)?;
    writeln!(file, "Miner's Change Amount (in BTC): {:.8}", miner_change_amount)?;
    writeln!(file, "Transaction Fees (in BTC): {:.8}", fee)?;
    writeln!(
        file,
        "Block height at which the transaction is confirmed: {}",
        block_height
    )?;
    writeln!(
        file,
        "Block hash at which the transaction is confirmed: {}",
        block_hash
    )?;

    Ok(())
}

