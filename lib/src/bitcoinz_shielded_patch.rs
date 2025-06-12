/// BitcoinZ Shielded Transaction Patcher
/// 
/// This module patches Zcash-format shielded transactions to work with BitcoinZ.
/// The key difference is the binding signature format:
/// - Zcash: sign(bsk, sighash) [32-byte message]
/// - BitcoinZ: sign(bsk, bvk || sighash) [64-byte message]

use blake2b_simd::{Params, State};
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::Write;

/// Patch a Zcash v4 transaction to use BitcoinZ binding signature
pub fn patch_shielded_binding_signature(
    tx_bytes: &[u8],
    bvk: &[u8; 32],
    binding_sig: &[u8; 64],
) -> Result<Vec<u8>, String> {
    if tx_bytes.len() < 64 {
        return Err("Transaction too small to contain binding signature".to_string());
    }
    
    // The binding signature is the last 64 bytes
    let sig_start = tx_bytes.len() - 64;
    
    // Create patched transaction
    let mut patched = tx_bytes.to_vec();
    
    // Replace the binding signature
    patched[sig_start..].copy_from_slice(binding_sig);
    
    Ok(patched)
}

/// Compute BitcoinZ sighash for shielded transactions
pub fn compute_bitcoinz_shielded_sighash(tx_data: &[u8]) -> [u8; 32] {
    let mut personal = [0u8; 16];
    personal[..12].copy_from_slice(b"BitcoinzSig\x19");
    let branch_id: u32 = 1991772603; // 0x76b809bb
    personal[12..16].copy_from_slice(&branch_id.to_le_bytes());
    
    let hash = Params::new()
        .hash_length(32)
        .personal(&personal)
        .to_state()
        .update(tx_data)
        .finalize();
    
    let mut result = [0u8; 32];
    result.copy_from_slice(hash.as_bytes());
    // BitcoinZ does NOT reverse the sighash
    result
}

/// Extract transaction data for sighash computation (excluding binding sig)
pub fn extract_tx_data_for_sighash(tx_bytes: &[u8]) -> Result<Vec<u8>, String> {
    if tx_bytes.len() < 64 {
        return Err("Transaction too small".to_string());
    }
    
    // Everything except the last 64 bytes (binding signature)
    Ok(tx_bytes[..tx_bytes.len() - 64].to_vec())
}

/// Create BitcoinZ binding signature message (bvk || sighash)
pub fn create_bitcoinz_binding_message(bvk: &[u8; 32], sighash: &[u8; 32]) -> [u8; 64] {
    let mut message = [0u8; 64];
    message[..32].copy_from_slice(bvk);
    message[32..].copy_from_slice(sighash);
    message
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sighash_computation() {
        let test_data = vec![0u8; 100];
        let sighash = compute_bitcoinz_shielded_sighash(&test_data);
        assert_eq!(sighash.len(), 32);
    }
    
    #[test]
    fn test_binding_message_creation() {
        let bvk = [1u8; 32];
        let sighash = [2u8; 32];
        let message = create_bitcoinz_binding_message(&bvk, &sighash);
        
        assert_eq!(&message[..32], &bvk);
        assert_eq!(&message[32..], &sighash);
    }
}