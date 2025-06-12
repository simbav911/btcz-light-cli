/// BitcoinZ Binding Signature Patcher
/// 
/// This module patches Zcash-format binding signatures to BitcoinZ format
/// after a transaction is built by the standard builder.

use blake2b_simd::Params;
use ff::Field;
use group::{Group, GroupEncoding};
use rand::thread_rng;

use zcash_primitives::{
    consensus::BlockHeight,
    sapling::{
        redjubjub::{PrivateKey, PublicKey, Signature},
        value::{CommitmentSum, ValueSum},
    },
    transaction::{
        components::sapling::{
            Bundle as SaplingBundle,
            SpendDescription,
            OutputDescription,
        },
        Transaction,
        TransactionData,
        TxVersion,
    },
};

/// Extract value commitment sum from sapling bundle
fn extract_value_commitment_sum<A>(
    sapling_bundle: &Option<SaplingBundle<A>>,
) -> Result<ValueSum, String> {
    if let Some(bundle) = sapling_bundle {
        let mut cv_sum = ValueSum::zero();
        
        // Add spend value commitments
        for spend in bundle.shielded_spends() {
            cv_sum = (cv_sum + spend.cv().to_inner()).ok_or("Value overflow")?;
        }
        
        // Subtract output value commitments
        for output in bundle.shielded_outputs() {
            cv_sum = (cv_sum - output.cv().to_inner()).ok_or("Value overflow")?;
        }
        
        Ok(cv_sum)
    } else {
        Ok(ValueSum::zero())
    }
}

/// Patch a transaction's binding signature to use BitcoinZ format
pub fn patch_bitcoinz_binding_signature(
    tx_bytes: &[u8],
    height: BlockHeight,
) -> Result<Vec<u8>, String> {
    // Parse the transaction
    let tx = Transaction::read(tx_bytes, zcash_primitives::consensus::BranchId::Nu5)
        .map_err(|e| format!("Failed to parse transaction: {:?}", e))?;
    
    // Only patch v4 (Sapling) transactions with shielded components
    match tx {
        Transaction::V4(tx_data) => {
            patch_v4_binding_signature(&tx_data, tx_bytes, height)
        }
        _ => Ok(tx_bytes.to_vec()), // Return unchanged for non-v4 transactions
    }
}

fn patch_v4_binding_signature(
    tx_data: &TransactionData<zcash_primitives::transaction::Authorized>,
    original_bytes: &[u8],
    height: BlockHeight,
) -> Result<Vec<u8>, String> {
    // Check if transaction has shielded components
    if tx_data.sapling_bundle().is_none() {
        return Ok(original_bytes.to_vec()); // No shielded components, no binding sig
    }
    
    // The binding signature is the last 64 bytes
    if original_bytes.len() < 64 {
        return Err("Transaction too small to contain binding signature".to_string());
    }
    
    let sig_start = original_bytes.len() - 64;
    
    // Extract value commitment sum to compute bsk
    let cv_sum = extract_value_commitment_sum(tx_data.sapling_bundle())?;
    
    // Convert cv_sum to bsk (binding signing key)
    let bsk = PrivateKey(cv_sum.to_inner());
    
    // Compute bvk (binding verification key) = bsk * G
    let bvk = PublicKey::from_private(&bsk, &jubjub::AffinePoint::generator());
    
    // Compute sighash for binding signature
    let sighash = compute_bitcoinz_sighash_v4(&original_bytes[..sig_start], height)?;
    
    // Create BitcoinZ binding signature message (bvk || sighash)
    let mut message = [0u8; 64];
    message[..32].copy_from_slice(&bvk.to_bytes());
    message[32..].copy_from_slice(&sighash);
    
    // Sign with BitcoinZ format
    let sig = bsk.sign(&message, &mut thread_rng());
    
    // Create patched transaction
    let mut patched = original_bytes.to_vec();
    patched[sig_start..].copy_from_slice(&sig.to_bytes());
    
    Ok(patched)
}

/// Compute BitcoinZ sighash for v4 transactions
fn compute_bitcoinz_sighash_v4(
    tx_data_no_sig: &[u8],
    _height: BlockHeight,
) -> Result<[u8; 32], String> {
    let mut personal = [0u8; 16];
    personal[..12].copy_from_slice(b"BitcoinzSig\x19");
    
    // BitcoinZ uses fixed branch ID
    let branch_id: u32 = 1991772603; // 0x76b809bb
    personal[12..16].copy_from_slice(&branch_id.to_le_bytes());
    
    let hash = Params::new()
        .hash_length(32)
        .personal(&personal)
        .to_state()
        .update(tx_data_no_sig)
        .finalize();
    
    let mut result = [0u8; 32];
    result.copy_from_slice(hash.as_bytes());
    
    // BitcoinZ does NOT reverse the hash
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sighash_computation() {
        let test_data = vec![0u8; 100];
        let height = BlockHeight::from_u32(1000000);
        let sighash = compute_bitcoinz_sighash_v4(&test_data, height).unwrap();
        assert_eq!(sighash.len(), 32);
    }
}