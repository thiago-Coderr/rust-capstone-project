use bitcoincore_rpc::{bitcoin::Address, bitcoin::Amount, bitcoin::Txid, Auth, Client, RpcApi};
use serde_json::json;
use std::fs::File;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let miner_rpc = Client::new(
        "http://127.0.0.1:18443",
        Auth::UserPass("user".to_string(), "pass".to_string()),
    )?;
    let trader_rpc = Client::new(
        "http://127.0.0.1:18444",
        Auth::UserPass("user".to_string(), "pass".to_string()),
    )?;

    let trader_address =
        trader_rpc.get_new_address(None, None)?.require_network(bitcoincore_rpc::bitcoin::Network::Regtest)?;
    let miner_address =
        miner_rpc.get_new_address(None, None)?.require_network(bitcoincore_rpc::bitcoin::Network::Regtest)?;

    // Generate block to fund the miner
    miner_rpc.generate_to_address(101, &miner_address)?;

    // Send 1 BTC to trader from miner
    let amount_to_send = Amount::from_btc(1.0)?;
    let txid = miner_rpc.send_to_address(&trader_address, amount_to_send, None, None, None, None, None, None)?;

    // Mine 1 block to confirm transaction
    miner_rpc.generate_to_address(1, &miner_address)?;

    // Get transaction info
    let tx = miner_rpc.get_transaction(&txid, Some(true))?;
    let raw_tx = miner_rpc.get_raw_transaction_info(&txid, None)?;

    let block_hash = raw_tx.blockhash.ok_or("Block hash not found")?;
    let block = miner_rpc.get_block_info(&block_hash)?;
    let block_height = block.height;

    let mut trader_output_amount = 0.0;
    let mut miner_change_amount = 0.0;
    let mut input_address: Option<Address> = None;

    for detail in &tx.details {
        match detail.category.as_str() {
            "receive" => trader_output_amount += detail.amount.to_btc(),
            "send" => miner_change_amount -= detail.amount.to_btc(),
            _ => {}
        }

        if input_address.is_none() && detail.address.is_some() {
            input_address = detail.address.clone();
        }
    }

    let fee = tx
        .details
        .iter()
        .map(|d| d.fee.unwrap_or(Amount::from_btc(0.0).unwrap()))
        .sum::<Amount>()
        .to_btc();

    let mut file = File::create("../out.txt")?;
    writeln!(file, "Transaction ID: {}", txid)?;
    writeln!(file, "Miner's Input Address: {:?}", input_address)?;
    writeln!(file, "Trader's Receiving Address: {}", trader_address)?;
    writeln!(
        file,
        "Trader's Received Amount (in BTC): {:.8}",
        trader_output_amount
    )?;
    writeln!(
        file,
        "Miner's Change Amount (in BTC): {:.8}",
        miner_change_amount
    )?;
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
