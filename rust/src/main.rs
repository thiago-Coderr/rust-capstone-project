use bitcoin::{Address, Txid};
use bitcoin::util::amount::Amount;
use bitcoincore_rpc::{Auth, Client, RpcApi};
use serde_json::json;
use std::fs::File;
use std::io::Write;

const RPC_URL: &str = "http://127.0.0.1:18443";
const RPC_USER: &str = "alice";
const RPC_PASS: &str = "password";

// Helper to create/load a wallet
fn load_wallet(rpc: &Client, wallet: &str) -> bitcoincore_rpc::Result<Client> {
    if rpc.list_wallets()?.contains(&wallet.to_string()) {
        return Client::new(
            &format!("{}/wallet/{}", RPC_URL, wallet),
            Auth::UserPass(RPC_USER.to_string(), RPC_PASS.to_string()),
        );
    }

    // If not found, create
    rpc.create_wallet(wallet, None, None, None, None)?;
    Client::new(
        &format!("{}/wallet/{}", RPC_URL, wallet),
        Auth::UserPass(RPC_USER.to_string(), RPC_PASS.to_string()),
    )
}

fn main() -> bitcoincore_rpc::Result<()> {
    let rpc = Client::new(
        RPC_URL,
        Auth::UserPass(RPC_USER.to_string(), RPC_PASS.to_string()),
    )?;

    // Load or create wallets
    let miner_rpc = load_wallet(&rpc, "Miner")?;
    let trader_rpc = load_wallet(&rpc, "Trader")?;

    // Generate 101 blocks to get spendable funds in Miner
    let miner_address = miner_rpc.get_new_address(None, None)?;
    miner_rpc.generate_to_address(101, &miner_address)?;

    // Trader gets a receiving address
    let trader_address = trader_rpc.get_new_address(None, None)?;

    // Send 20 BTC from Miner to Trader
    let txid = miner_rpc.send_to_address(
        &trader_address,
        Amount::from_btc(20.0).unwrap(),
        None, None, None, None, None, None,
    )?;

    // Transaction is in mempool, confirm it
    miner_rpc.generate_to_address(1, &miner_address)?;

    // Retrieve transaction info
    let tx = miner_rpc.get_raw_transaction_info(&txid, None)?;
    let block_hash = tx.blockhash.unwrap();
    let block = miner_rpc.get_block_info(&block_hash)?;
    let block_height = block.height;

    // Inputs
    let input_tx = miner_rpc.get_raw_transaction_info(&tx.vin[0].txid, None)?;
    let miner_input_value = input_tx.vout[tx.vin[0].vout as usize].value;

    let input_address = input_tx.vout[tx.vin[0].vout as usize]
        .script_pub_key
        .addresses
        .as_ref()
        .unwrap()
        .first()
        .unwrap()
        .clone();

    // Outputs: find trader and change
    let mut trader_output_address = String::new();
    let mut trader_output_amount = 0.0;
    let mut miner_change_address = String::new();
    let mut miner_change_amount = 0.0;

    for vout in &tx.vout {
        let address = vout.script_pub_key.addresses
            .as_ref()
            .unwrap()
            .first()
            .unwrap()
            .clone();
        if address == trader_address.to_string() {
            trader_output_address = address;
            trader_output_amount = vout.value;
        } else {
            miner_change_address = address;
            miner_change_amount = vout.value;
        }
    }

    // Calculate fee
    let fee = miner_input_value - (trader_output_amount + miner_change_amount);

    // Output to file (outside of /rust)
    let mut file = File::create("../out.txt")?;
    writeln!(file, "{}", txid)?;
    writeln!(file, "{}", input_address)?;
    writeln!(file, "{:.8}", miner_input_value)?;
    writeln!(file, "{}", trader_output_address)?;
    writeln!(file, "{:.8}", trader_output_amount)?;
    writeln!(file, "{}", miner_change_address)?;
    writeln!(file, "{:.8}", miner_change_amount)?;
    writeln!(file, "{:.8}", fee)?;
    writeln!(file, "{}", block_height)?;
    writeln!(file, "{}", block_hash)?;

    println!("Transaction info written to ../out.txt ✅");

    Ok(())
}
