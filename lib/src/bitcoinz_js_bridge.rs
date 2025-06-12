/// BitcoinZ JavaScript Bridge
/// 
/// This module interfaces with bitcore-lib-btcz through Node.js to generate
/// BitcoinZ-compatible shielded transaction components.

use std::process::Command;
use serde::{Serialize, Deserialize};
use serde_json;
use hex;
use zcash_primitives::{
    consensus::{BlockHeight, Parameters},
    legacy::TransparentAddress,
    transaction::components::{Amount, OutPoint, TxOut},
    sapling::PaymentAddress,
    memo::MemoBytes,
};
use zcash_client_backend::encoding::{AddressCodec, encode_payment_address};
use secp256k1::SecretKey;

#[derive(Debug, Serialize)]
struct JsRequest {
    action: String,
    params: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct JsResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tx_hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<ShieldedOutputJs>,
}

#[derive(Debug, Deserialize)]
struct ShieldedOutputJs {
    cv: String,
    cmu: String,
    ephemeral_key: String,
    enc_ciphertext: String,
    out_ciphertext: String,
    zkproof: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsTransactionResult {
    success: bool,
    txid: Option<String>,
    hex: Option<String>,
    size: Option<usize>,
    error: Option<String>,
}

/// Call the JavaScript bridge to process a request
fn call_js_bridge(request: &JsRequest) -> Result<JsResponse, String> {
    let request_json = serde_json::to_string(request)
        .map_err(|e| format!("Failed to serialize request: {}", e))?;
    
    println!("BitcoinZ JS Bridge: Calling with request: {}", request_json);
    
    // Find the script path
    let script_path = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?
        .join("btcz-shielded-bridge.js");
    
    if !script_path.exists() {
        return Err(format!("BitcoinZ shielded bridge script not found at {:?}", script_path));
    }
    
    let output = Command::new("node")
        .arg(script_path)
        .arg(&request_json)
        .output()
        .map_err(|e| format!("Failed to execute Node.js: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Node.js script failed: {}", stderr));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("BitcoinZ JS Bridge: Response: {}", stdout);
    
    serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse response: {} (output: {})", e, stdout))
}

/// Generate a BitcoinZ-compatible shielded output using the JS bridge
pub fn generate_shielded_output<P: Parameters>(
    params: &P,
    to_address: &PaymentAddress,
    amount: Amount,
    memo: &MemoBytes,
) -> Result<ShieldedOutputComponents, String> {
    // Encode the payment address
    let to_addr_str = encode_payment_address(
        params.hrp_sapling_payment_address(),
        to_address
    );
    
    let params = serde_json::json!({
        "to_address": to_addr_str,
        "amount": u64::from(amount),
        "memo": hex::encode(memo.as_slice()),
    });
    
    let request = JsRequest {
        action: "create_output".to_string(),
        params,
    };
    
    let response = call_js_bridge(&request)?;
    
    if !response.success {
        return Err(response.error.unwrap_or_else(|| "Unknown error".to_string()));
    }
    
    let output = response.output
        .ok_or_else(|| "No output in response".to_string())?;
    
    Ok(ShieldedOutputComponents {
        cv: hex::decode(&output.cv)
            .map_err(|e| format!("Failed to decode cv: {}", e))?,
        cmu: hex::decode(&output.cmu)
            .map_err(|e| format!("Failed to decode cmu: {}", e))?,
        ephemeral_key: hex::decode(&output.ephemeral_key)
            .map_err(|e| format!("Failed to decode ephemeral_key: {}", e))?,
        enc_ciphertext: hex::decode(&output.enc_ciphertext)
            .map_err(|e| format!("Failed to decode enc_ciphertext: {}", e))?,
        out_ciphertext: hex::decode(&output.out_ciphertext)
            .map_err(|e| format!("Failed to decode out_ciphertext: {}", e))?,
        zkproof: hex::decode(&output.zkproof)
            .map_err(|e| format!("Failed to decode zkproof: {}", e))?,
    })
}

/// Components of a shielded output
pub struct ShieldedOutputComponents {
    pub cv: Vec<u8>,
    pub cmu: Vec<u8>,
    pub ephemeral_key: Vec<u8>,
    pub enc_ciphertext: Vec<u8>,
    pub out_ciphertext: Vec<u8>,
    pub zkproof: Vec<u8>,
}

/// Build a BitcoinZ transaction using the JavaScript library (legacy)
pub fn build_bitcoinz_js_tx<P: Parameters>(
    params: &P,
    inputs: Vec<(OutPoint, TxOut, SecretKey)>,
    outputs: Vec<(TransparentAddress, Amount)>,
    _height: BlockHeight,
) -> Result<Vec<u8>, String> {
    
    if inputs.len() != 1 || outputs.len() > 2 {
        return Err("JavaScript bridge currently only supports single input transactions".to_string());
    }
    
    let (outpoint, _txout, sk) = &inputs[0];
    let (to_addr, amount) = &outputs[0];
    
    // Convert secret key to WIF format
    let sk_bytes = sk.as_ref();
    let wif = secret_key_to_wif(sk_bytes)?;
    
    // Get the to address
    let to_address = to_addr.encode(params);
    
    // Amount in satoshis
    let amount_sats = u64::from(*amount);
    
    // Build UTXO JSON
    let utxo_json = serde_json::json!({
        "txid": hex::encode(outpoint.hash()),
        "vout": outpoint.n(),
        "scriptPubKey": hex::encode(&_txout.script_pubkey.0),
        "satoshis": u64::from(_txout.value)
    });
    
    // Find the Node.js script path
    let script_path = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?
        .join("bitcoinz-tx-builder.js");
    
    if !script_path.exists() {
        return Err(format!("BitcoinZ transaction builder script not found at {:?}", script_path));
    }
    
    
    // Execute the Node.js script
    let output = Command::new("node")
        .arg(script_path)
        .arg(&wif)
        .arg(&to_address)
        .arg(amount_sats.to_string())
        .arg(utxo_json.to_string())
        .output()
        .map_err(|e| format!("Failed to execute Node.js script: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Node.js script failed: {}", stderr));
    }
    
    // Parse the output
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Find the JSON output
    if let Some(json_start) = stdout.find("---JSON OUTPUT---") {
        let json_str = &stdout[json_start + 17..].trim();
        
        let result: JsTransactionResult = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse JSON output: {}", e))?;
        
        if result.success {
            if let Some(hex) = result.hex {
                let tx_bytes = hex::decode(&hex)
                    .map_err(|e| format!("Failed to decode transaction hex: {}", e))?;
                
                
                return Ok(tx_bytes);
            } else {
                return Err("No transaction hex in successful result".to_string());
            }
        } else {
            return Err(format!("JavaScript error: {}", result.error.unwrap_or_default()));
        }
    }
    
    Err("Failed to parse Node.js output".to_string())
}

/// Convert secret key bytes to WIF format
fn secret_key_to_wif(sk_bytes: &[u8]) -> Result<String, String> {
    use base58::ToBase58;
    use sha2::{Sha256, Digest};
    
    // BitcoinZ mainnet private key prefix is 0x80
    let mut data = vec![0x80];
    data.extend_from_slice(sk_bytes);
    data.push(0x01); // Compressed pubkey flag
    
    // Double SHA256 for checksum
    let hash1 = Sha256::digest(&data);
    let hash2 = Sha256::digest(&hash1);
    
    // Append first 4 bytes of checksum
    data.extend_from_slice(&hash2[..4]);
    
    Ok(data.to_base58())
}

#[derive(Debug)]
pub struct TransparentInput {
    pub txid: String,
    pub vout: u32,
    pub script_pubkey: String,
    pub amount: u64,
}

#[derive(Debug)]
pub struct TransparentOutput {
    pub address: String,
    pub amount: u64,
}

#[derive(Debug)]
pub struct ShieldedOutput {
    pub address: String,
    pub amount: u64,
    pub memo: Vec<u8>,
}