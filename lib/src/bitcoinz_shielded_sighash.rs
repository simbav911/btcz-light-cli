/// BitcoinZ Shielded Transaction Sighash
/// 
/// This module implements the proper sighash computation for BitcoinZ shielded
/// transactions, following ZIP-243 with BitcoinZ-specific modifications

use blake2b_simd::Params;
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::Write;

use zcash_primitives::{
    consensus::BlockHeight,
    transaction::{
        components::{
            sapling::{OutputDescription, SpendDescription, Authorization},
            transparent::{self, TxIn, TxOut},
            Amount,
        },
    },
};

/// BitcoinZ constants
const CONSENSUS_BRANCH_ID: u32 = 1991772603; // 0x76b809bb
const SAPLING_TX_VERSION: i32 = 4;
const BITCOINZ_VERSION_GROUP_ID: u32 = 0x892f2085;

/// Personalization strings
const ZCASH_PREVOUTS_HASH_PERSONALIZATION: &[u8; 16] = b"ZcashPrevoutHash";
const ZCASH_SEQUENCE_HASH_PERSONALIZATION: &[u8; 16] = b"ZcashSequencHash";
const ZCASH_OUTPUTS_HASH_PERSONALIZATION: &[u8; 16] = b"ZcashOutputsHash";
const ZCASH_JOINSPLITS_HASH_PERSONALIZATION: &[u8; 16] = b"ZcashJSplitsHash";
const ZCASH_SHIELDED_SPENDS_HASH_PERSONALIZATION: &[u8; 16] = b"ZcashSSpendsHash";
const ZCASH_SHIELDED_OUTPUTS_HASH_PERSONALIZATION: &[u8; 16] = b"ZcashSOutputHash";
const ZCASH_SAPLING_SIGHASH_PERSONALIZATION_PREFIX: &[u8; 12] = b"ZcashSigHash";

/// Compute the sighash for a BitcoinZ shielded transaction
pub fn compute_shielded_sighash(
    transparent_inputs: &[(transparent::OutPoint, TxOut)],
    transparent_outputs: &[TxOut],
    shielded_spends: &[SpendDescription<impl Authorization>],
    shielded_outputs: &[OutputDescription<impl Authorization>],
    value_balance: i64,
    tx_metadata: TxMetadata,
) -> Result<[u8; 32], String> {
    let mut data = Vec::new();
    
    // 1. Header
    let header = 0x80000000u32 | (SAPLING_TX_VERSION as u32);
    data.write_u32::<LittleEndian>(header)?;
    
    // 2. Version group ID
    data.write_u32::<LittleEndian>(BITCOINZ_VERSION_GROUP_ID)?;
    
    // 3. Prevouts hash
    let prevouts_hash = if transparent_inputs.is_empty() {
        [0u8; 32]
    } else {
        compute_prevouts_hash(transparent_inputs)?
    };
    data.write_all(&prevouts_hash)?;
    
    // 4. Sequence hash
    let sequence_hash = if transparent_inputs.is_empty() {
        [0u8; 32]
    } else {
        compute_sequence_hash(transparent_inputs.len())?
    };
    data.write_all(&sequence_hash)?;
    
    // 5. Outputs hash
    let outputs_hash = if transparent_outputs.is_empty() {
        [0u8; 32]
    } else {
        compute_transparent_outputs_hash(transparent_outputs)?
    };
    data.write_all(&outputs_hash)?;
    
    // 6. JoinSplits hash (empty for v4)
    data.write_all(&[0u8; 32])?;
    
    // 7. Shielded spends hash
    let shielded_spends_hash = if shielded_spends.is_empty() {
        [0u8; 32]
    } else {
        compute_shielded_spends_hash(shielded_spends)?
    };
    data.write_all(&shielded_spends_hash)?;
    
    // 8. Shielded outputs hash
    let shielded_outputs_hash = if shielded_outputs.is_empty() {
        [0u8; 32]
    } else {
        compute_shielded_outputs_hash(shielded_outputs)?
    };
    data.write_all(&shielded_outputs_hash)?;
    
    // 9. Lock time
    data.write_u32::<LittleEndian>(tx_metadata.lock_time)?;
    
    // 10. Expiry height
    data.write_u32::<LittleEndian>(tx_metadata.expiry_height)?;
    
    // 11. Value balance
    data.write_i64::<LittleEndian>(value_balance)?;
    
    // 12. Hash type
    data.write_u32::<LittleEndian>(tx_metadata.hash_type)?;
    
    // If sighash type is for a specific input, add input-specific data
    if let Some(input_data) = tx_metadata.input_data {
        // Prevout
        data.write_all(input_data.prevout.hash())?;
        data.write_u32::<LittleEndian>(input_data.prevout.n())?;
        
        // Script code
        write_compact_size(&mut data, input_data.script_code.len() as u64)?;
        data.write_all(&input_data.script_code)?;
        
        // Amount
        data.write_u64::<LittleEndian>(u64::from(input_data.amount))?;
        
        // Sequence
        data.write_u32::<LittleEndian>(input_data.sequence)?;
    }
    
    // Compute final hash with BitcoinZ personalization
    let mut personalization = [0u8; 16];
    personalization[..12].copy_from_slice(ZCASH_SAPLING_SIGHASH_PERSONALIZATION_PREFIX);
    personalization[12..16].copy_from_slice(&CONSENSUS_BRANCH_ID.to_le_bytes());
    
    let hash = Params::new()
        .hash_length(32)
        .personal(&personalization)
        .to_state()
        .update(&data)
        .finalize();
    
    let mut result = [0u8; 32];
    result.copy_from_slice(hash.as_bytes());
    
    // BitcoinZ: Do NOT reverse the bytes
    Ok(result)
}

/// Transaction metadata for sighash computation
pub struct TxMetadata {
    pub lock_time: u32,
    pub expiry_height: u32,
    pub hash_type: u32,
    pub input_data: Option<InputData>,
}

/// Input-specific data for sighash computation
pub struct InputData {
    pub prevout: transparent::OutPoint,
    pub script_code: Vec<u8>,
    pub amount: Amount,
    pub sequence: u32,
}

/// Compute hash of all transparent input prevouts
fn compute_prevouts_hash(
    inputs: &[(transparent::OutPoint, TxOut)],
) -> Result<[u8; 32], String> {
    let mut data = Vec::new();
    
    for (outpoint, _) in inputs {
        data.write_all(outpoint.hash())?;
        data.write_u32::<LittleEndian>(outpoint.n())?;
    }
    
    let hash = Params::new()
        .hash_length(32)
        .personal(ZCASH_PREVOUTS_HASH_PERSONALIZATION)
        .to_state()
        .update(&data)
        .finalize();
    
    let mut result = [0u8; 32];
    result.copy_from_slice(hash.as_bytes());
    Ok(result)
}

/// Compute hash of all input sequences
fn compute_sequence_hash(input_count: usize) -> Result<[u8; 32], String> {
    let mut data = Vec::new();
    
    for _ in 0..input_count {
        data.write_u32::<LittleEndian>(0xfffffffe)?;
    }
    
    let hash = Params::new()
        .hash_length(32)
        .personal(ZCASH_SEQUENCE_HASH_PERSONALIZATION)
        .to_state()
        .update(&data)
        .finalize();
    
    let mut result = [0u8; 32];
    result.copy_from_slice(hash.as_bytes());
    Ok(result)
}

/// Compute hash of all transparent outputs
fn compute_transparent_outputs_hash(outputs: &[TxOut]) -> Result<[u8; 32], String> {
    let mut data = Vec::new();
    
    for output in outputs {
        data.write_u64::<LittleEndian>(u64::from(output.value))?;
        write_compact_size(&mut data, output.script_pubkey.0.len() as u64)?;
        data.write_all(&output.script_pubkey.0)?;
    }
    
    let hash = Params::new()
        .hash_length(32)
        .personal(ZCASH_OUTPUTS_HASH_PERSONALIZATION)
        .to_state()
        .update(&data)
        .finalize();
    
    let mut result = [0u8; 32];
    result.copy_from_slice(hash.as_bytes());
    Ok(result)
}

/// Compute hash of all shielded spends
fn compute_shielded_spends_hash<A: Authorization>(
    spends: &[SpendDescription<A>],
) -> Result<[u8; 32], String> {
    let mut data = Vec::new();
    
    for spend in spends {
        // cv
        data.write_all(&spend.cv.to_bytes())?;
        // anchor
        data.write_all(&spend.anchor.to_repr())?;
        // nullifier
        data.write_all(&spend.nullifier.0)?;
        // rk
        data.write_all(&spend.rk.to_bytes())?;
        // zkproof
        data.write_all(&spend.zkproof)?;
    }
    
    let hash = Params::new()
        .hash_length(32)
        .personal(ZCASH_SHIELDED_SPENDS_HASH_PERSONALIZATION)
        .to_state()
        .update(&data)
        .finalize();
    
    let mut result = [0u8; 32];
    result.copy_from_slice(hash.as_bytes());
    Ok(result)
}

/// Compute hash of all shielded outputs
fn compute_shielded_outputs_hash<A: Authorization>(
    outputs: &[OutputDescription<A>],
) -> Result<[u8; 32], String> {
    let mut data = Vec::new();
    
    for output in outputs {
        // cv
        data.write_all(&output.cv.to_bytes())?;
        // cmu
        data.write_all(&output.cmu.to_repr())?;
        // ephemeral_key
        data.write_all(&output.ephemeral_key.to_bytes())?;
        // enc_ciphertext
        data.write_all(&output.enc_ciphertext)?;
        // out_ciphertext
        data.write_all(&output.out_ciphertext)?;
        // zkproof
        data.write_all(&output.zkproof)?;
    }
    
    let hash = Params::new()
        .hash_length(32)
        .personal(ZCASH_SHIELDED_OUTPUTS_HASH_PERSONALIZATION)
        .to_state()
        .update(&data)
        .finalize();
    
    let mut result = [0u8; 32];
    result.copy_from_slice(hash.as_bytes());
    Ok(result)
}

/// Write variable-length integer
fn write_compact_size(writer: &mut Vec<u8>, size: u64) -> Result<(), String> {
    if size < 0xfd {
        writer.push(size as u8);
    } else if size <= 0xffff {
        writer.push(0xfd);
        writer.write_u16::<LittleEndian>(size as u16)?;
    } else if size <= 0xffffffff {
        writer.push(0xfe);
        writer.write_u32::<LittleEndian>(size as u32)?;
    } else {
        writer.push(0xff);
        writer.write_u64::<LittleEndian>(size)?;
    }
    Ok(())
}

// Re-export WriteBytesExt trait for error conversion
trait WriteExt: Write {
    fn write_u16<T: byteorder::ByteOrder>(&mut self, n: u16) -> Result<(), String> {
        WriteBytesExt::write_u16::<T>(self, n)
            .map_err(|e| format!("Write error: {}", e))
    }
    
    fn write_u32<T: byteorder::ByteOrder>(&mut self, n: u32) -> Result<(), String> {
        WriteBytesExt::write_u32::<T>(self, n)
            .map_err(|e| format!("Write error: {}", e))
    }
    
    fn write_u64<T: byteorder::ByteOrder>(&mut self, n: u64) -> Result<(), String> {
        WriteBytesExt::write_u64::<T>(self, n)
            .map_err(|e| format!("Write error: {}", e))
    }
    
    fn write_i64<T: byteorder::ByteOrder>(&mut self, n: i64) -> Result<(), String> {
        WriteBytesExt::write_i64::<T>(self, n)
            .map_err(|e| format!("Write error: {}", e))
    }
}

impl<W: Write> WriteExt for W {}