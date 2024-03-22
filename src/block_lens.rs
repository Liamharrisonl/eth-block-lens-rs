use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde_json::{json, Value};

fn now_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
}

fn rpc_call(url: &str, method: &str, params: Value) -> anyhow::Result<Value> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    let req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    });
    let res: Value = client.post(url).json(&req).send()?.json()?;
    if let Some(err) = res.get("error") {
        anyhow::bail!("rpc error: {}", err);
    }
    Ok(res["result"].clone())
}

fn hex_to_u64(h: &Value) -> u64 {
    u64::from_str_radix(h.as_str().unwrap_or("0x0").trim_start_matches("0x"), 16).unwrap_or(0)
}

pub fn run() -> anyhow::Result<()> {
    // Usage: block_lens <RPC_URL> [<blockNumber|latest>]
    let mut args = std::env::args().skip(1);
    let url = args.next().unwrap_or("http://localhost:8545".into());
    let target = args.next().unwrap_or("latest".into());

    // Resolve block number
    let block_num_hex = if target == "latest" {
        rpc_call(&url, "eth_blockNumber", json!([]))?
    } else if target.starts_with("0x") {
        Value::String(target)
    } else {
        Value::String(format!("0x{:x}", target.parse::<u64>().unwrap_or(0)))
    };

    let t0 = now_ms();
    let block = rpc_call(&url, "eth_getBlockByNumber", json!([block_num_hex, true]))?;
    let t1 = now_ms();

    // Extract metrics
    let gas_used = hex_to_u64(&block["gasUsed"]);
    let gas_limit = hex_to_u64(&block["gasLimit"]);
    let txs = block["transactions"].as_array().map(|a| a.len()).unwrap_or(0);
    let base_fee = hex_to_u64(&block["baseFeePerGas"]);
    let number = hex_to_u64(&block["number"]);
    let ts = hex_to_u64(&block["timestamp"]);

    println!("number,timestamp,tx_count,gas_used,gas_limit,base_fee_gwei,latency_ms");
    println!("{},{},{},{},{},{},{}",
        number,
        ts,
        txs,
        gas_used,
        gas_limit,
        (base_fee as f64) / 1e9,
        (t1 as i128 - t0 as i128)
    );

    Ok(())
}
