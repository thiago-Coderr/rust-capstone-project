use bitcoincore_rpc::{Auth, Client, RpcApi};
use bitcoincore_rpc::bitcoin::{Amount, Txid};
use std::fs::File;
use std::io::Write;


const RPC_URL_MINER: &str = "http://127.0.0.1:18443/wallet/Miner";
const RPC_URL_TRADER: &str = "http://127.0.0.1:18443/wallet/Trader";
const RPC_USER: &str = "alice";
const RPC_PASS: &str = "password";

fn main() -> bitcoincore_rpc::Result<()> {
    // Connect to both wallets
    let miner_rpc = Client::new(RPC_URL_MINER, Auth::UserPass(RPC_USER.into(), RPC_PASS.into()))?;
    let trader_rpc = Client::new(RPC_URL_TRADER, Auth::UserPass(RPC_USER.into(), RPC_PASS.into()))?;

    // Create wallets if they don’t exist (ignore error if already exists)
    let root_rpc = Client::new("http://127.0.0.1:18443", Auth::UserPass(RPC_USER.into(), RPC_PASS.into()))?;
    let _ = root_rpc.create_wallet("Miner", Some(false), None, None, None);
    let _ = root_rpc.create_wallet("Trader", Some(false), None, None, None);

    // Generate 103 blocks to get spendable funds for Miner
    let miner_mining_address = miner_rpc.get_new_address(None, None)?;
    miner_rpc.generate_to_address(103, &miner_mining_address)?;

    // Generate Trader receiving address
    let trader_address = trader_rpc.get_new_address(None, None)?;

    // Send 20 BTC to Trader
    let send_amount = Amount::from_btc(20.0)?;
    let txid = miner_rpc.send_to_address(&trader_address, send_amount, None, None, None, None, None, None)?;

    // Mine 1 block to confirm
    miner_rpc.generate_to_address(1, &miner_mining_address)?;

    // Fetch transaction details
    let tx_result = miner_rpc.get_transaction(&txid, Some(true))?;
    let decoded_tx = tx_result.info.decoded().unwrap();
    let blockhash = tx_result.info.blockhash.unwrap();
    let block = miner_rpc.get_block_header_info(&blockhash)?;
    let blockheight = block.height;
    let fee_btc = tx_result.info.fee.unwrap().to_btc(); // negative value

    // Extract input details (we assume 1 input)
    let vin = &decoded_tx.vin[0];
    let input_txid = vin.txid;
    let input_vout = vin.vout;

    let input_tx = miner_rpc.get_raw_transaction_info(&input_txid, None)?;
    let input_vout_info = &input_tx.vout[input_vout as usize];
    let miner_input_address = input_vout_info.script_pub_key.addresses.as_ref().unwrap()[0].clone();
    let miner_input_amount = input_vout_info.value;

    // Identify outputs
    let mut miner_change_address = String::new();
    let mut miner_change_amount = 0.0;
    let mut trader_output_amount = 0.0;

    for vout in &decoded_tx.vout {
        let addr = &vout.script_pub_key.addresses.as_ref().unwrap()[0];
        if addr == &trader_address.to_string() {
            trader_output_amount = vout.value;
        } else {
            miner_change_address = addr.clone();
            miner_change_amount = vout.value;
        }
    }

    // Write to out.txt (one line per attribute)
    let mut file = File::create("../../out.txt")?;
    writeln!(file, "{}", txid)?;                         // txid
    writeln!(file, "{}", miner_input_address)?;         // miner input address
    writeln!(file, "{}", miner_input_amount)?;          // miner input amount
    writeln!(file, "{}", trader_address)?;              // trader output address
    writeln!(file, "{}", trader_output_amount)?;        // trader output amount
    writeln!(file, "{}", miner_change_address)?;        // miner change address
    writeln!(file, "{}", miner_change_amount)?;         // miner change amount
    writeln!(file, "{}", fee_btc)?;                     // fee (already negative)
    writeln!(file, "{}", blockheight)?;                 // block height
    writeln!(file, "{}", blockhash)?;                   // block hash

    Ok(())
}

