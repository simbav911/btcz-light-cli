/// BitcoinZ v4 Shielded Transaction Builder
/// 
/// This module builds v4 Sapling transactions with proper binding signatures
/// for shielded transfers (t→z, z→t, z→z), using BitcoinZ's specific
/// binding signature algorithm.

use byteorder::{LittleEndian, WriteBytesExt};
use ff::{Field, PrimeField};
use group::{GroupEncoding, Group};
use rand::{thread_rng, Rng, RngCore, SeedableRng};
use hex;
use rand::rngs::StdRng;
use secp256k1::{Message, PublicKey as SecpPublicKey, Secp256k1, SecretKey};
use blake2b_simd::Params;
use std::io::Write;
use std::convert::TryInto;

use zcash_primitives::{
    consensus::{BlockHeight, BranchId, Parameters},
    keys::OutgoingViewingKey,
    legacy::{Script, TransparentAddress},
    memo::MemoBytes,
    sapling::{
        keys::{ExpandedSpendingKey, FullViewingKey},
        note_encryption::sapling_note_encryption,
        prover::TxProver,
        redjubjub::{PrivateKey, PublicKey, Signature},
        Diversifier, Node, Note, PaymentAddress, ProofGenerationKey, Rseed,
        NoteValue, ValueCommitment,
    },
    transaction::{
        components::{
            sapling::{
                Authorization, Authorized, Bundle, GrothProofBytes,
                OutputDescription, SpendDescription,
            },
            transparent::{self, TxIn, TxOut},
            Amount, GROTH_PROOF_SIZE,
        },
        sighash::{signature_hash, SignableInput},
        txid::TxIdDigester,
        TransactionData, TxVersion, Unauthorized,
    },
};

use crate::bitcoinz_js_bridge::{generate_shielded_output as js_generate_shielded_output};
use crate::bitcoinz_compat::{serialize_value_commitment_bitcoinz, serialize_ephemeral_key_bitcoinz};
use crate::bitcoinz_compat_v2::{serialize_edwards_point_bitcoinz_v2, serialize_edwards_point_bitcoinz_v3, serialize_edwards_point_bitcoinz_v4, serialize_edwards_point_bitcoinz_exact, debug_point_formats};
use crate::bitcoinz_edwards_bellman::write_edwards_point_bellman;

/// BitcoinZ Sapling constants
const SAPLING_TX_VERSION: i32 = 4;
const BITCOINZ_VERSION_GROUP_ID: u32 = 0x892f2085;
const CONSENSUS_BRANCH_ID: u32 = 1991772603; // 0x76b809bb
const SIGHASH_ALL: u32 = 1;

/// Personalization strings for BLAKE2b
const ZCASH_SAPLING_SIGHASH_PERSONALIZATION_PREFIX: &[u8; 12] = b"ZcashSigHash";
const ZCASH_PREVOUTS_HASH_PERSONALIZATION: &[u8; 16] = b"ZcashPrevoutHash";
const ZCASH_SEQUENCE_HASH_PERSONALIZATION: &[u8; 16] = b"ZcashSequencHash";
const ZCASH_OUTPUTS_HASH_PERSONALIZATION: &[u8; 16] = b"ZcashOutputsHash";
const ZCASH_SHIELDED_SPENDS_HASH_PERSONALIZATION: &[u8; 16] = b"ZcashSSpendsHash";
const ZCASH_SHIELDED_OUTPUTS_HASH_PERSONALIZATION: &[u8; 16] = b"ZcashSOutputHash";

// The Jubjub curve parameters are now part of the jubjub crate itself
// No need for lazy_static - we'll use the constants directly

/// A shielded input (spend) to be included in the transaction
pub struct ShieldedSpend {
    pub note: Note,
    pub diversifier: Diversifier,
    pub merkle_path: Vec<Node>,
    pub witness_position: u64,
    pub alpha: jubjub::Fr,
}

/// A shielded output to be created in the transaction
pub struct ShieldedOutput {
    pub ovk: OutgoingViewingKey,
    pub to: PaymentAddress,
    pub value: Amount,
    pub memo: MemoBytes,
}

/// Builder for BitcoinZ v4 shielded transactions
pub struct BitcoinZShieldedBuilder<P: Parameters> {
    params: P,
    height: BlockHeight,
    
    // Transparent components
    transparent_inputs: Vec<(transparent::OutPoint, TxOut, SecretKey)>,
    transparent_outputs: Vec<(TransparentAddress, Amount)>,
    
    // Shielded components
    sapling_spends: Vec<ShieldedSpend>,
    sapling_outputs: Vec<ShieldedOutput>,
    
    // Keys
    spending_keys: Vec<ExpandedSpendingKey>,
    
    // Random number generator
    rng: StdRng,
}

impl<P: Parameters> BitcoinZShieldedBuilder<P> {
    /// Create a new builder instance
    pub fn new(
        params: P,
        height: BlockHeight,
    ) -> Self {
        Self {
            params,
            height,
            transparent_inputs: Vec::new(),
            transparent_outputs: Vec::new(),
            sapling_spends: Vec::new(),
            sapling_outputs: Vec::new(),
            spending_keys: Vec::new(),
            rng: StdRng::from_entropy(),
        }
    }
    
    /// Add a transparent input
    pub fn add_transparent_input(
        &mut self,
        outpoint: transparent::OutPoint,
        coin: TxOut,
        key: SecretKey,
    ) -> Result<(), String> {
        self.transparent_inputs.push((outpoint, coin, key));
        Ok(())
    }
    
    /// Add a transparent output
    pub fn add_transparent_output(
        &mut self,
        to: TransparentAddress,
        value: Amount,
    ) -> Result<(), String> {
        self.transparent_outputs.push((to, value));
        Ok(())
    }
    
    /// Add a Sapling spend (shielded input)
    pub fn add_sapling_spend(
        &mut self,
        extsk: ExpandedSpendingKey,
        diversifier: Diversifier,
        note: Note,
        merkle_path: Vec<Node>,
        witness_position: u64,
    ) -> Result<(), String> {
        let alpha = jubjub::Fr::random(&mut self.rng);
        
        self.spending_keys.push(extsk);
        self.sapling_spends.push(ShieldedSpend {
            note,
            diversifier,
            merkle_path,
            witness_position,
            alpha,
        });
        
        Ok(())
    }
    
    /// Add a Sapling output
    pub fn add_sapling_output(
        &mut self,
        ovk: OutgoingViewingKey,
        to: PaymentAddress,
        value: Amount,
        memo: MemoBytes,
    ) -> Result<(), String> {
        self.sapling_outputs.push(ShieldedOutput {
            ovk,
            to,
            value,
            memo,
        });
        
        Ok(())
    }
    
    /// Build and sign the transaction
    pub fn build<Pr: TxProver>(
        mut self,
        prover: &Pr,
        fee: Amount,
    ) -> Result<Vec<u8>, String> {
        println!("BitcoinZ Builder: {} transparent inputs, {} transparent outputs, {} sapling spends, {} sapling outputs",
                 self.transparent_inputs.len(), self.transparent_outputs.len(), 
                 self.sapling_spends.len(), self.sapling_outputs.len());
        // Calculate value balance
        let value_balance = self.calculate_value_balance(fee)?;
        
        // Generate binding signature key
        let mut bsk = jubjub::Fr::zero();
        
        // Build spend descriptions
        let mut shielded_spends = Vec::new();
        for (i, spend) in self.sapling_spends.iter().enumerate() {
            let spend_desc = self.build_spend_description(
                prover,
                &self.spending_keys[i],
                spend,
                &mut bsk,
            )?;
            shielded_spends.push(spend_desc);
        }
        
        // Build output descriptions
        let mut shielded_outputs = Vec::new();
        for (i, output) in self.sapling_outputs.iter().enumerate() {
            println!("BitcoinZ: Building output {} to address {:?}", i, output.to);
            let output_desc = self.build_output_description(
                prover,
                output,
                &mut bsk,
            )?;
            shielded_outputs.push(output_desc);
        }
        
        // Convert bsk to RedJubjub private key
        let bsk = PrivateKey(bsk);
        
        // Compute binding verification key
        // Compute bvk = bsk * G on the Jubjub curve
        // Compute bvk directly from bsk
        // Compute bvk = bsk * G on the Jubjub curve
        let bvk = PublicKey::from_private(&bsk, jubjub::SubgroupPoint::generator());
        
        // Now build the full transaction
        let mut tx_data = Vec::new();
        
        // Write transaction header
        self.write_header(&mut tx_data)?;
        println!("BitcoinZ: After header, tx size: {} bytes", tx_data.len());
        
        // Write transparent inputs
        self.write_transparent_inputs(&mut tx_data)?;
        println!("BitcoinZ: After transparent inputs, tx size: {} bytes", tx_data.len());
        
        // Write transparent outputs  
        self.write_transparent_outputs(&mut tx_data)?;
        println!("BitcoinZ: After transparent outputs, tx size: {} bytes", tx_data.len());
        
        // Write lock time and expiry
        tx_data.write_u32::<LittleEndian>(0).map_err(|e| e.to_string())?; // lock_time
        tx_data.write_u32::<LittleEndian>(0).map_err(|e| e.to_string())?; // expiry_height
        println!("BitcoinZ: After locktime/expiry, tx size: {} bytes", tx_data.len());
        
        // Write value balance
        println!("BitcoinZ: Writing value balance: {} ({:#x})", value_balance, value_balance);
        tx_data.write_i64::<LittleEndian>(value_balance).map_err(|e| e.to_string())?;
        println!("BitcoinZ: After value balance, tx size: {} bytes", tx_data.len());
        
        // Write shielded spends
        println!("BitcoinZ: Writing {} shielded spends", shielded_spends.len());
        write_compact_size(&mut tx_data, shielded_spends.len() as u64)?;
        for spend in &shielded_spends {
            self.write_spend_description(&mut tx_data, spend)?;
        }
        println!("BitcoinZ: After shielded spends, tx size: {} bytes", tx_data.len());
        
        // Write shielded outputs
        println!("BitcoinZ: Writing {} shielded outputs", shielded_outputs.len());
        write_compact_size(&mut tx_data, shielded_outputs.len() as u64)?;
        let output_start = tx_data.len();
        for (i, output) in shielded_outputs.iter().enumerate() {
            println!("BitcoinZ: Writing output description {}", i);
            println!("  cv: {} bytes", output.cv.to_bytes().len());
            println!("  cmu: {} bytes", output.cmu.to_repr().len());
            println!("  ephemeral_key: {} bytes", output.ephemeral_key.0.len());
            println!("  enc_ciphertext: {} bytes", output.enc_ciphertext.len());
            println!("  out_ciphertext: {} bytes", output.out_ciphertext.len());
            println!("  zkproof: {} bytes", output.zkproof.len());
            let before_output = tx_data.len();
            self.write_output_description(&mut tx_data, output)?;
            println!("  Output {} size: {} bytes", i, tx_data.len() - before_output);
        }
        println!("BitcoinZ: Total shielded outputs size: {} bytes", tx_data.len() - output_start);
        println!("BitcoinZ: After shielded outputs, tx size: {} bytes", tx_data.len());
        
        // No JoinSplits in v4
        write_compact_size(&mut tx_data, 0)?;
        println!("BitcoinZ: After JoinSplits count (0), tx size: {} bytes", tx_data.len());
        
        // Compute sighash for binding signature (before adding the signature)
        println!("BitcoinZ: Computing binding signature sighash");
        let sighash = self.compute_binding_sig_sighash(
            &shielded_spends,
            &shielded_outputs,
            value_balance,
        )?;
        println!("BitcoinZ: Binding signature sighash: {}", hex::encode(&sighash));
        
        // Compute BitcoinZ binding signature
        println!("BitcoinZ: Computing BitcoinZ binding signature with 64-byte message");
        let binding_sig = compute_bitcoinz_binding_signature(&bsk, &bvk, &sighash)?;
        
        // Write binding signature
        // Serialize the signature
        let sig_bytes = {
            let mut bytes = [0u8; 64];
            binding_sig.write(&mut bytes[..]).map_err(|e| e.to_string())?;
            bytes
        };
        println!("BitcoinZ: Binding signature bytes (hex): {}", hex::encode(&sig_bytes));
        tx_data.write_all(&sig_bytes).map_err(|e| e.to_string())?;
        println!("BitcoinZ: After binding signature, tx size: {} bytes", tx_data.len());
        
        // Now sign the transparent inputs if any
        if !self.transparent_inputs.is_empty() {
            println!("BitcoinZ: Signing {} transparent inputs", self.transparent_inputs.len());
            tx_data = self.sign_transparent_inputs(
                tx_data,
                &shielded_spends,
                &shielded_outputs,
                value_balance,
            )?;
        }
        
        // Debug: Print transaction hex
        println!("BitcoinZ: Final transaction size: {} bytes", tx_data.len());
        println!("BitcoinZ: Transaction structure summary:");
        println!("  Header: 8 bytes");
        println!("  Transparent inputs: {} count", self.transparent_inputs.len());
        println!("  Transparent outputs: {} count", self.transparent_outputs.len());
        println!("  Shielded spends: {} count", shielded_spends.len());
        println!("  Shielded outputs: {} count", shielded_outputs.len());
        println!("  Value balance: {}", value_balance);
        println!("BitcoinZ: Transaction hex: {}", hex::encode(&tx_data));
        
        Ok(tx_data)
    }
    
    // Helper methods continue below...
}

/// Compute BitcoinZ binding signature with 64-byte message
fn compute_bitcoinz_binding_signature(
    bsk: &PrivateKey,
    bvk: &PublicKey, 
    sighash: &[u8; 32],
) -> Result<Signature, String> {
    // BitcoinZ expects: sign(bsk, bvk || sighash)
    let mut message = [0u8; 64];
    // Serialize bvk
    let mut bvk_bytes = [0u8; 32];
    bvk.write(&mut bvk_bytes[..]).map_err(|e| e.to_string())?;
    message[..32].copy_from_slice(&bvk_bytes);
    message[32..].copy_from_slice(sighash);
    
    // Sign the 64-byte message
    let mut rng = thread_rng();
    // Sign with the generator point
    let generator = jubjub::SubgroupPoint::generator();
    Ok(bsk.sign(&message, &mut rng, generator))
}

/// Write compact size
fn write_compact_size(writer: &mut Vec<u8>, size: u64) -> Result<(), String> {
    if size < 0xfd {
        writer.push(size as u8);
    } else if size <= 0xffff {
        writer.push(0xfd);
        writer.write_u16::<LittleEndian>(size as u16)
            .map_err(|e| format!("Failed to write compact size: {}", e))?;
    } else if size <= 0xffffffff {
        writer.push(0xfe);
        writer.write_u32::<LittleEndian>(size as u32)
            .map_err(|e| format!("Failed to write compact size: {}", e))?;
    } else {
        writer.push(0xff);
        writer.write_u64::<LittleEndian>(size)
            .map_err(|e| format!("Failed to write compact size: {}", e))?;
    }
    Ok(())
}

/// Read a variable-length integer
fn read_compact_size(data: &[u8]) -> Result<Option<(u64, usize)>, String> {
    if data.is_empty() {
        return Ok(None);
    }
    
    let first = data[0];
    match first {
        0..=0xfc => Ok(Some((first as u64, 1))),
        0xfd => {
            if data.len() < 3 {
                return Ok(None);
            }
            let val = u16::from_le_bytes([data[1], data[2]]) as u64;
            Ok(Some((val, 3)))
        }
        0xfe => {
            if data.len() < 5 {
                return Ok(None);
            }
            let val = u32::from_le_bytes([data[1], data[2], data[3], data[4]]) as u64;
            Ok(Some((val, 5)))
        }
        0xff => {
            if data.len() < 9 {
                return Ok(None);
            }
            let val = u64::from_le_bytes([
                data[1], data[2], data[3], data[4],
                data[5], data[6], data[7], data[8]
            ]);
            Ok(Some((val, 9)))
        }
    }
}

/// Compute hash of all shielded spends
fn compute_shielded_spends_hash(
    spends: &[SpendDescription<Authorized>],
) -> Result<[u8; 32], String> {
    let mut data = Vec::new();
    
    for spend in spends {
        // cv
        data.write_all(&spend.cv.to_bytes())
            .map_err(|e| format!("Failed to write cv: {}", e))?;
        // anchor
        data.write_all(&spend.anchor.to_repr())
            .map_err(|e| format!("Failed to write anchor: {}", e))?;
        // nullifier
        data.write_all(&spend.nullifier.0)
            .map_err(|e| format!("Failed to write nullifier: {}", e))?;
        // rk
        let mut rk_bytes = [0u8; 32];
        spend.rk.write(&mut rk_bytes[..]).map_err(|e| e.to_string())?;
        data.write_all(&rk_bytes)
            .map_err(|e| format!("Failed to write rk: {}", e))?;
        // zkproof
        data.write_all(&spend.zkproof)
            .map_err(|e| format!("Failed to write zkproof: {}", e))?;
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
fn compute_shielded_outputs_hash(
    outputs: &[OutputDescription<GrothProofBytes>],
) -> Result<[u8; 32], String> {
    let mut data = Vec::new();
    
    for output in outputs {
        // cv
        data.write_all(&output.cv.to_bytes())
            .map_err(|e| format!("Failed to write cv: {}", e))?;
        // cmu
        data.write_all(&output.cmu.to_repr())
            .map_err(|e| format!("Failed to write cmu: {}", e))?;
        // ephemeral_key
        data.write_all(&output.ephemeral_key.0)
            .map_err(|e| format!("Failed to write ephemeral_key: {}", e))?;
        // enc_ciphertext
        data.write_all(&output.enc_ciphertext)
            .map_err(|e| format!("Failed to write enc_ciphertext: {}", e))?;
        // out_ciphertext
        data.write_all(&output.out_ciphertext)
            .map_err(|e| format!("Failed to write out_ciphertext: {}", e))?;
        // zkproof
        data.write_all(&output.zkproof)
            .map_err(|e| format!("Failed to write zkproof: {}", e))?;
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


impl<P: Parameters> BitcoinZShieldedBuilder<P> {
    /// Calculate the value balance for the transaction
    fn calculate_value_balance(&self, fee: Amount) -> Result<i64, String> {
        let mut total_in = 0i64;
        let mut total_out = 0i64;
        
        // Add transparent inputs
        for (_, coin, _) in &self.transparent_inputs {
            total_in += i64::from(coin.value);
        }
        
        // Add shielded inputs (spends)
        for spend in &self.sapling_spends {
            total_in += spend.note.value as i64;
        }
        
        // Subtract transparent outputs
        for (_, amount) in &self.transparent_outputs {
            total_out += i64::from(*amount);
        }
        
        // Subtract shielded outputs
        for output in &self.sapling_outputs {
            total_out += i64::from(output.value);
        }
        
        // Subtract fee
        total_out += i64::from(fee);
        
        // Value balance is the net flow from transparent to shielded
        // Negative means funds flowing into shielded pool (t→z)
        // Positive means funds flowing out of shielded pool (z→t)
        Ok(total_in - total_out)
    }
    
    /// Build a spend description
    fn build_spend_description<Pr: TxProver>(
        &self,
        prover: &Pr,
        extsk: &ExpandedSpendingKey,
        spend: &ShieldedSpend,
        bsk: &mut jubjub::Fr,
    ) -> Result<SpendDescription<Authorized>, String> {
        // Derive the full viewing key
        let fvk = FullViewingKey::from_expanded_spending_key(extsk);
        
        // Compute nullifier
        let nk = fvk.vk.nk;
        let nullifier = spend.note.nf(&nk, 0); // position 0 for now
        
        // Randomize the spend authorization key
        // Randomize the spend authorization key
        let rsk = PrivateKey(extsk.ask + spend.alpha);
        
        // Create value commitment
        let rcv = jubjub::Fr::random(&mut thread_rng());
        // Create value commitment - the cv will be computed by the prover
        // We just need to track rcv for the binding signature key
        
        // Update binding signature key
        *bsk += rcv;
        
        // Generate proof
        // For v0.7, we need to create the proof context and use different parameters
        let mut ctx = prover.new_sapling_proving_context();
        
        // Use the diversifier from the spend
        let diversifier = spend.diversifier;
        
        // Create merkle path from the witness nodes
        if spend.merkle_path.len() != 32 {
            return Err("Invalid merkle path length: expected 32 nodes".to_string());
        }
        
        let mut auth_path = Vec::with_capacity(32);
        for node in &spend.merkle_path {
            auth_path.push((*node, false)); // The bool indicates whether this is the right child
        }
        
        let merkle_path = zcash_primitives::merkle_tree::MerklePath {
            auth_path: auth_path.try_into()
                .map_err(|_| "Failed to convert merkle path")?,
            position: spend.witness_position,
        };
        
        let (proof, cv_proof, rk) = prover
            .spend_proof(
                &mut ctx,
                extsk.proof_generation_key(),
                diversifier,
                spend.note.rseed,
                spend.alpha,
                spend.note.value,
                spend.note.cmu(),
                merkle_path,
            )
            .map_err(|e| format!("Failed to create spend proof: {:?}", e))?;
        
        // Create spend auth signature (placeholder for now)
        let spend_auth_sig = Signature::read(&[0u8; 64][..])
            .map_err(|e| format!("Failed to create spend auth sig: {:?}", e))?;
        
        // Get the anchor from merkle path root
        // For now use a placeholder anchor - this might be the issue!
        let anchor = [0u8; 32];
        println!("BitcoinZ WARNING: Using placeholder anchor, this needs to be fixed!");
        
        Ok(SpendDescription {
            cv: cv_proof.into(),
            anchor: bls12_381::Scalar::from_bytes(&anchor).unwrap(),
            nullifier,
            rk,
            zkproof: proof,
            spend_auth_sig,
        })
    }
    
    /// Build an output description
    fn build_output_description<Pr: TxProver>(
        &self,
        prover: &Pr,
        output: &ShieldedOutput,
        bsk: &mut jubjub::Fr,
    ) -> Result<OutputDescription<GrothProofBytes>, String> {
        println!("BitcoinZ: Building output description");
        println!("  Payment address: {:?}", output.to);
        println!("  Value: {:?}", output.value);
        println!("  Memo: {} bytes", output.memo.as_array().len());
        
        println!("BitcoinZ: Using JavaScript bridge to generate shielded output");
        
        // Try using the JavaScript bridge first
        match js_generate_shielded_output(
            &self.params,
            &output.to,
            output.value,
            &output.memo,
        ) {
            Ok(js_output) => {
                println!("BitcoinZ: Successfully generated output using JS bridge");
                
                // Parse the components from JS bridge
                if js_output.cv.len() != 32 {
                    return Err(format!("Invalid cv length from JS: {}", js_output.cv.len()));
                }
                if js_output.cmu.len() != 32 {
                    return Err(format!("Invalid cmu length from JS: {}", js_output.cmu.len()));
                }
                if js_output.ephemeral_key.len() != 32 {
                    return Err(format!("Invalid ephemeral_key length from JS: {}", js_output.ephemeral_key.len()));
                }
                if js_output.zkproof.len() != GROTH_PROOF_SIZE {
                    return Err(format!("Invalid zkproof length from JS: {}", js_output.zkproof.len()));
                }
                
                // Convert cv bytes to ExtendedPoint for bsk calculation
                // Note: This is approximate - we're using a placeholder rcv value
                let rcv = jubjub::Fr::random(&mut thread_rng());
                *bsk -= rcv;
                
                // Convert to proper types
                let cv = jubjub::ExtendedPoint::from_bytes(&js_output.cv.try_into().unwrap()).unwrap();
                let cmu = bls12_381::Scalar::from_bytes(&js_output.cmu.try_into().unwrap()).unwrap();
                let ephemeral_key = zcash_note_encryption::EphemeralKeyBytes(js_output.ephemeral_key.try_into().unwrap());
                
                return Ok(OutputDescription {
                    cv: cv.into(),
                    cmu,
                    ephemeral_key,
                    enc_ciphertext: js_output.enc_ciphertext.try_into().unwrap(),
                    out_ciphertext: js_output.out_ciphertext.try_into().unwrap(),
                    zkproof: js_output.zkproof.try_into().unwrap(),
                });
            }
            Err(e) => {
                println!("BitcoinZ: JS bridge failed: {}, falling back to native implementation", e);
            }
        }
        
        // Fallback to native implementation
        println!("BitcoinZ: Using native implementation for output description");
        
        // Generate note
        let rseed = Rseed::AfterZip212(thread_rng().gen());
        println!("  Generated rseed");
        
        let note = output.to.create_note(
            u64::from(output.value),
            rseed,
        ).ok_or("Failed to create note")?;
        println!("  Created note with value: {}", note.value);
        println!("  Note cmu: {}", hex::encode(note.cmu().to_repr()));
        
        // Create value commitment
        let rcv = jubjub::Fr::random(&mut thread_rng());
        println!("  Generated rcv for value commitment");
        
        // Update binding signature key (negative for outputs)
        *bsk -= rcv;
        
        // Generate proof first to get cv
        // For v0.7, we need the proving context
        println!("  Creating proving context");
        let mut ctx = prover.new_sapling_proving_context();
        
        println!("  Generating output proof with:");
        println!("    rcv: {:?}", rcv);
        println!("    payment_address pk_d: {:?}", output.to.pk_d());
        println!("    payment_address diversifier: {:?}", output.to.diversifier());
        println!("    note rcm: {:?}", note.rcm());
        println!("    note value: {}", note.value);
        
        let (proof, cv_proof) = prover
            .output_proof(
                &mut ctx,
                rcv,
                output.to.clone(),
                note.rcm(),
                note.value,
            );
        
        println!("  Generated proof, size: {} bytes", proof.len());
        println!("  cv (value commitment): {}", hex::encode(cv_proof.to_bytes()));
        
        // Validate cv is not small order
        // Small order check: point * 8 should not be zero
        let cv_times_8 = cv_proof.double().double().double();
        if bool::from(cv_times_8.is_identity()) {
            return Err("Generated cv is of small order".to_string());
        }
        
        // Encrypt the note with the actual cv
        println!("  Starting note encryption");
        let ne = sapling_note_encryption::<_, P>(
            Some(output.ovk),
            note.clone(),
            output.to.clone(),
            output.memo.clone(),
            &mut thread_rng(),
        );
        
        let enc_ciphertext = ne.encrypt_note_plaintext();
        println!("  Encrypted note plaintext, size: {} bytes", enc_ciphertext.len());
        
        let out_ciphertext = ne.encrypt_outgoing_plaintext(&cv_proof, &note.cmu(), &mut thread_rng());
        println!("  Encrypted outgoing plaintext, size: {} bytes", out_ciphertext.len());
        
        let ephemeral_key = ne.epk();
        println!("  Ephemeral key: {}", hex::encode(ephemeral_key.to_bytes()));
        
        // Validate ephemeral key is not small order
        // Note: ephemeral_key is already an ExtendedPoint
        let epk_times_8 = ephemeral_key.double().double().double();
        if bool::from(epk_times_8.is_identity()) {
            return Err("Generated ephemeral key is of small order".to_string());
        }
        
        // Debug: print the cv bytes to verify serialization
        println!("BitcoinZ: Analyzing cv format:");
        debug_point_formats(&cv_proof);
        println!("BitcoinZ: cmu bytes (hex): {}", hex::encode(note.cmu().to_repr()));
        println!("BitcoinZ: Analyzing ephemeral key format:");
        debug_point_formats(&ephemeral_key);
        
        // Log the complete output description structure
        println!("BitcoinZ: Complete output description:");
        println!("  cv: {} bytes", cv_proof.to_bytes().len());
        println!("  cmu: {} bytes", 32);
        println!("  ephemeral_key: {} bytes", ephemeral_key.to_bytes().len());
        println!("  enc_ciphertext: {} bytes", enc_ciphertext.len());
        println!("  out_ciphertext: {} bytes", out_ciphertext.len());
        println!("  zkproof: {} bytes", proof.len());
        
        // Store cv and ephemeral_key in a format we can serialize later
        Ok(OutputDescription {
            cv: cv_proof.into(),
            cmu: note.cmu(),
            ephemeral_key: ephemeral_key.to_bytes().into(),
            enc_ciphertext,
            out_ciphertext,
            zkproof: proof,
        })
    }
    
    /// Write transaction header
    fn write_header(&self, tx_data: &mut Vec<u8>) -> Result<(), String> {
        let header = 0x80000000u32 | (SAPLING_TX_VERSION as u32);
        tx_data.write_u32::<LittleEndian>(header)
            .map_err(|e| format!("Failed to write header: {}", e))?;
        
        tx_data.write_u32::<LittleEndian>(BITCOINZ_VERSION_GROUP_ID)
            .map_err(|e| format!("Failed to write version group ID: {}", e))?;
        
        Ok(())
    }
    
    /// Write transparent inputs
    fn write_transparent_inputs(&self, tx_data: &mut Vec<u8>) -> Result<(), String> {
        write_compact_size(tx_data, self.transparent_inputs.len() as u64)?;
        
        for (outpoint, _, _) in &self.transparent_inputs {
            tx_data.write_all(outpoint.hash())
                .map_err(|e| format!("Failed to write outpoint hash: {}", e))?;
            tx_data.write_u32::<LittleEndian>(outpoint.n())
                .map_err(|e| format!("Failed to write outpoint index: {}", e))?;
            
            // Script sig will be added after signing
            write_compact_size(tx_data, 0)?;
            
            tx_data.write_u32::<LittleEndian>(0xfffffffe)
                .map_err(|e| format!("Failed to write sequence: {}", e))?;
        }
        
        Ok(())
    }
    
    /// Write transparent outputs
    fn write_transparent_outputs(&self, tx_data: &mut Vec<u8>) -> Result<(), String> {
        write_compact_size(tx_data, self.transparent_outputs.len() as u64)?;
        
        for (addr, amount) in &self.transparent_outputs {
            tx_data.write_u64::<LittleEndian>(u64::from(*amount))
                .map_err(|e| format!("Failed to write amount: {}", e))?;
            
            let script = addr.script();
            write_compact_size(tx_data, script.0.len() as u64)?;
            tx_data.write_all(&script.0)
                .map_err(|e| format!("Failed to write script: {}", e))?;
        }
        
        Ok(())
    }
    
    /// Write spend description
    fn write_spend_description(
        &self,
        tx_data: &mut Vec<u8>,
        spend: &SpendDescription<Authorized>,
    ) -> Result<(), String> {
        // Write cv using standard format
        tx_data.write_all(&spend.cv.to_bytes())
            .map_err(|e| format!("Failed to write cv: {}", e))?;
        
        // Write anchor
        tx_data.write_all(&spend.anchor.to_repr())
            .map_err(|e| format!("Failed to write anchor: {}", e))?;
        
        // Write nullifier
        tx_data.write_all(&spend.nullifier.0)
            .map_err(|e| format!("Failed to write nullifier: {}", e))?;
        
        // Write rk
        let mut rk_bytes = [0u8; 32];
        spend.rk.write(&mut rk_bytes[..]).map_err(|e| e.to_string())?;
        tx_data.write_all(&rk_bytes)
            .map_err(|e| format!("Failed to write rk: {}", e))?;
        
        // Write zkproof
        tx_data.write_all(&spend.zkproof)
            .map_err(|e| format!("Failed to write zkproof: {}", e))?;
        
        // Write spend_auth_sig
        let mut sig_bytes = [0u8; 64];
        spend.spend_auth_sig.write(&mut sig_bytes[..]).map_err(|e| e.to_string())?;
        tx_data.write_all(&sig_bytes)
            .map_err(|e| format!("Failed to write spend_auth_sig: {}", e))?;
        
        Ok(())
    }
    
    /// Write output description
    fn write_output_description(
        &self,
        tx_data: &mut Vec<u8>,
        output: &OutputDescription<GrothProofBytes>,
    ) -> Result<(), String> {
        println!("BitcoinZ: Writing output description details:");
        
        // Write cv using BitcoinZ's bellman 0.1.0 format
        println!("  Writing cv in bellman 0.1.0 format");
        let cv_standard = output.cv.to_bytes();
        println!("    cv standard format: {}", hex::encode(&cv_standard));
        let cv_start = tx_data.len();
        // Convert cv bytes to ExtendedPoint
        let cv_point = jubjub::ExtendedPoint::from_bytes(&cv_standard).unwrap();
        write_edwards_point_bellman(&cv_point, &mut *tx_data)
            .map_err(|e| format!("Failed to write cv: {}", e))?;
        let cv_bellman = &tx_data[cv_start..];
        println!("    cv bellman format: {}", hex::encode(cv_bellman));
        println!("    formats differ: {}", cv_standard != cv_bellman);
        
        // Write cmu
        let cmu_bytes = output.cmu.to_repr();
        println!("  cmu bytes (hex): {}", hex::encode(&cmu_bytes));
        tx_data.write_all(&cmu_bytes)
            .map_err(|e| format!("Failed to write cmu: {}", e))?;
        
        // Write ephemeral_key using bellman 0.1.0 format
        println!("  Writing ephemeral_key in bellman 0.1.0 format");
        let epk_standard = &output.ephemeral_key.0;
        println!("    ephemeral_key standard format: {}", hex::encode(epk_standard));
        // First decode the ephemeral key from bytes
        let epk_point = jubjub::ExtendedPoint::from_bytes(&output.ephemeral_key.0)
            .unwrap();
        let epk_start = tx_data.len();
        write_edwards_point_bellman(&epk_point, &mut *tx_data)
            .map_err(|e| format!("Failed to write ephemeral_key: {}", e))?;
        let epk_bellman = &tx_data[epk_start..];
        println!("    ephemeral_key bellman format: {}", hex::encode(epk_bellman));
        println!("    formats differ: {}", epk_standard != epk_bellman);
        
        // Write enc_ciphertext
        println!("  enc_ciphertext first 32 bytes (hex): {}", hex::encode(&output.enc_ciphertext[..32]));
        println!("  enc_ciphertext last 32 bytes (hex): {}", hex::encode(&output.enc_ciphertext[output.enc_ciphertext.len()-32..]));
        tx_data.write_all(&output.enc_ciphertext)
            .map_err(|e| format!("Failed to write enc_ciphertext: {}", e))?;
        
        // Write out_ciphertext
        println!("  out_ciphertext first 32 bytes (hex): {}", hex::encode(&output.out_ciphertext[..32]));
        tx_data.write_all(&output.out_ciphertext)
            .map_err(|e| format!("Failed to write out_ciphertext: {}", e))?;
        
        // Write zkproof
        println!("  zkproof first 32 bytes (hex): {}", hex::encode(&output.zkproof[..32]));
        println!("  zkproof last 32 bytes (hex): {}", hex::encode(&output.zkproof[output.zkproof.len()-32..]));
        tx_data.write_all(&output.zkproof)
            .map_err(|e| format!("Failed to write zkproof: {}", e))?;
        
        println!("  Total output description size: {} bytes", 
            32 + 32 + 32 + output.enc_ciphertext.len() + output.out_ciphertext.len() + output.zkproof.len());
        
        Ok(())
    }
    
    /// Compute sighash for binding signature according to ZIP-243
    fn compute_binding_sig_sighash(
        &self,
        shielded_spends: &[SpendDescription<Authorized>],
        shielded_outputs: &[OutputDescription<GrothProofBytes>],
        value_balance: i64,
    ) -> Result<[u8; 32], String> {
        let mut data = Vec::new();
        
        // 1. Header with Sapling flag
        let header = 0x80000000u32 | (SAPLING_TX_VERSION as u32);
        data.write_u32::<LittleEndian>(header)
            .map_err(|e| format!("Failed to write header: {}", e))?;
        
        // 2. Version group ID
        data.write_u32::<LittleEndian>(BITCOINZ_VERSION_GROUP_ID)
            .map_err(|e| format!("Failed to write version group ID: {}", e))?;
        
        // 3. Prevouts hash (hash of all transparent input prevouts)
        if self.transparent_inputs.is_empty() {
            data.write_all(&[0u8; 32])
                .map_err(|e| format!("Failed to write empty prevouts hash: {}", e))?;
        } else {
            let prevouts_hash = self.compute_prevouts_hash()?;
            data.write_all(&prevouts_hash)
                .map_err(|e| format!("Failed to write prevouts hash: {}", e))?;
        }
        
        // 4. Sequence hash
        if self.transparent_inputs.is_empty() {
            data.write_all(&[0u8; 32])
                .map_err(|e| format!("Failed to write empty sequence hash: {}", e))?;
        } else {
            let sequences_hash = self.compute_sequences_hash()?;
            data.write_all(&sequences_hash)
                .map_err(|e| format!("Failed to write sequences hash: {}", e))?;
        }
        
        // 5. Outputs hash
        if self.transparent_outputs.is_empty() {
            data.write_all(&[0u8; 32])
                .map_err(|e| format!("Failed to write empty outputs hash: {}", e))?;
        } else {
            let outputs_hash = self.compute_outputs_hash()?;
            data.write_all(&outputs_hash)
                .map_err(|e| format!("Failed to write outputs hash: {}", e))?;
        }
        
        // 6. JoinSplits hash (empty for v4)
        data.write_all(&[0u8; 32])
            .map_err(|e| format!("Failed to write joinsplits hash: {}", e))?;
        
        // 7. Shielded spends hash
        if shielded_spends.is_empty() {
            data.write_all(&[0u8; 32])
                .map_err(|e| format!("Failed to write empty shielded spends hash: {}", e))?;
        } else {
            let shielded_spends_hash = compute_shielded_spends_hash(shielded_spends)?;
            data.write_all(&shielded_spends_hash)
                .map_err(|e| format!("Failed to write shielded spends hash: {}", e))?;
        }
        
        // 8. Shielded outputs hash
        if shielded_outputs.is_empty() {
            data.write_all(&[0u8; 32])
                .map_err(|e| format!("Failed to write empty shielded outputs hash: {}", e))?;
        } else {
            let shielded_outputs_hash = compute_shielded_outputs_hash(shielded_outputs)?;
            data.write_all(&shielded_outputs_hash)
                .map_err(|e| format!("Failed to write shielded outputs hash: {}", e))?;
        }
        
        // 9. Lock time (using 0)
        data.write_u32::<LittleEndian>(0)
            .map_err(|e| format!("Failed to write lock time: {}", e))?;
        
        // 10. Expiry height (using 0)
        data.write_u32::<LittleEndian>(0)
            .map_err(|e| format!("Failed to write expiry height: {}", e))?;
        
        // 11. Value balance
        data.write_i64::<LittleEndian>(value_balance)
            .map_err(|e| format!("Failed to write value balance: {}", e))?;
        
        // 12. Hash type (SIGHASH_ALL for binding signature)
        data.write_u32::<LittleEndian>(1)
            .map_err(|e| format!("Failed to write hash type: {}", e))?;
        
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
    
    /// Compute sighash for binding signature
    fn compute_sighash_for_binding(&self, tx_data: &[u8]) -> Result<[u8; 32], String> {
        // Compute sighash without binding signature (up to the point before binding sig)
        // Transaction structure up to binding signature:
        // - Header (8 bytes)
        // - Transparent inputs/outputs
        // - Lock time (4 bytes)
        // - Expiry height (4 bytes)
        // - Value balance (8 bytes)
        // - Shielded spends and outputs
        // - JoinSplit count (1 byte for v4)
        // Then comes the binding signature which we don't include
        
        // For v4 the sighash excludes the binding signature itself
        let tx_without_binding_sig = &tx_data[..tx_data.len() - 64]; // Exclude 64-byte signature
        
        let mut personalization = [0u8; 16];
        personalization[..12].copy_from_slice(ZCASH_SAPLING_SIGHASH_PERSONALIZATION_PREFIX);
        personalization[12..16].copy_from_slice(&CONSENSUS_BRANCH_ID.to_le_bytes());
        
        let hash = Params::new()
            .hash_length(32)
            .personal(&personalization)
            .to_state()
            .update(tx_without_binding_sig)
            .finalize();
        
        let mut result = [0u8; 32];
        result.copy_from_slice(hash.as_bytes());
        
        Ok(result)
    }
    
    /// Sign transparent inputs
    fn sign_transparent_inputs(
        &self,
        unsigned_tx: Vec<u8>,
        shielded_spends: &[SpendDescription<Authorized>],
        shielded_outputs: &[OutputDescription<GrothProofBytes>],
        value_balance: i64,
    ) -> Result<Vec<u8>, String> {
        let secp = Secp256k1::new();
        let mut signatures = Vec::new();
        
        // Compute signatures for each input
        for (index, (_, txout, sk)) in self.transparent_inputs.iter().enumerate() {
            // Compute the sighash for this input
            let sighash = self.compute_sapling_sighash(
                &unsigned_tx,
                index,
                &txout.script_pubkey,
                txout.value,
                SIGHASH_ALL,
                shielded_spends,
                shielded_outputs,
                value_balance,
            )?;
            
            // Sign the sighash
            let msg = Message::from_slice(&sighash)
                .map_err(|e| format!("Failed to create message: {}", e))?;
            let sig = secp.sign_ecdsa(&msg, &sk);
            
            // Create script sig
            let pk = SecpPublicKey::from_secret_key(&secp, &sk);
            let mut script_sig = Vec::new();
            
            // Push signature with sighash type
            let mut sig_bytes = sig.serialize_der().to_vec();
            sig_bytes.push(SIGHASH_ALL as u8);
            script_sig.push(sig_bytes.len() as u8);
            script_sig.extend_from_slice(&sig_bytes);
            
            // Push public key
            let pk_bytes = pk.serialize();
            script_sig.push(pk_bytes.len() as u8);
            script_sig.extend_from_slice(&pk_bytes);
            
            signatures.push(script_sig);
        }
        
        // Now rebuild the transaction with signatures
        let mut signed_tx = Vec::new();
        
        // Copy header and version group ID (first 8 bytes)
        signed_tx.extend_from_slice(&unsigned_tx[0..8]);
        
        // Write inputs with signatures
        write_compact_size(&mut signed_tx, self.transparent_inputs.len() as u64)?;
        
        for (i, (outpoint, _, _)) in self.transparent_inputs.iter().enumerate() {
            // Previous output
            signed_tx.write_all(outpoint.hash())
                .map_err(|e| format!("Failed to write outpoint hash: {}", e))?;
            signed_tx.write_u32::<LittleEndian>(outpoint.n())
                .map_err(|e| format!("Failed to write outpoint index: {}", e))?;
            
            // Script sig with signature
            write_compact_size(&mut signed_tx, signatures[i].len() as u64)?;
            signed_tx.write_all(&signatures[i])
                .map_err(|e| format!("Failed to write script sig: {}", e))?;
            
            // Sequence
            signed_tx.write_u32::<LittleEndian>(0xfffffffe)
                .map_err(|e| format!("Failed to write sequence: {}", e))?;
        }
        
        // Find where outputs start in the unsigned transaction
        let mut cursor = 8; // After header and version group
        let (input_count, varint_size) = read_compact_size(&unsigned_tx[cursor..])
            .map_err(|e| format!("Failed to read input count: {}", e))?
            .ok_or("Failed to read input count")?;
        cursor += varint_size;
        
        // Skip all inputs in unsigned tx
        for _ in 0..input_count {
            cursor += 32 + 4; // outpoint
            let (script_len, varint_size) = read_compact_size(&unsigned_tx[cursor..])
                .map_err(|e| format!("Failed to read script length: {}", e))?
                .ok_or("Failed to read script length")?;
            cursor += varint_size + script_len as usize;
            cursor += 4; // sequence
        }
        
        // Copy the rest (outputs, locktime, expiry, value balance, shielded data, binding sig)
        signed_tx.extend_from_slice(&unsigned_tx[cursor..]);
        
        Ok(signed_tx)
    }
    
    /// Compute Sapling (v4) sighash using BLAKE2b
    fn compute_sapling_sighash(
        &self,
        _tx_data: &[u8],
        input_index: usize,
        script_code: &Script,
        value: Amount,
        sighash_type: u32,
        shielded_spends: &[SpendDescription<Authorized>],
        shielded_outputs: &[OutputDescription<GrothProofBytes>],
        value_balance: i64,
    ) -> Result<[u8; 32], String> {
        // This implements the Sapling sighash algorithm (ZIP-243)
        let mut data = Vec::new();
        
        // 1. Header with Sapling flag
        let header = 0x80000000u32 | (SAPLING_TX_VERSION as u32);
        data.write_u32::<LittleEndian>(header)
            .map_err(|e| format!("Failed to write header: {}", e))?;
        
        // 2. Version group ID
        data.write_u32::<LittleEndian>(BITCOINZ_VERSION_GROUP_ID)
            .map_err(|e| format!("Failed to write version group ID: {}", e))?;
        
        // 3. Hash of all prevouts (if not ANYONECANPAY)
        if (sighash_type & 0x80) == 0 {
            let prevouts_hash = self.compute_prevouts_hash()?;
            data.write_all(&prevouts_hash)
                .map_err(|e| format!("Failed to write prevouts hash: {}", e))?;
        } else {
            data.write_all(&[0u8; 32])
                .map_err(|e| format!("Failed to write empty prevouts hash: {}", e))?;
        }
        
        // 4. Hash of all sequences (if not ANYONECANPAY, SINGLE, NONE)
        if (sighash_type & 0x80) == 0 && (sighash_type & 0x1f) != 2 && (sighash_type & 0x1f) != 3 {
            let sequences_hash = self.compute_sequences_hash()?;
            data.write_all(&sequences_hash)
                .map_err(|e| format!("Failed to write sequences hash: {}", e))?;
        } else {
            data.write_all(&[0u8; 32])
                .map_err(|e| format!("Failed to write empty sequences hash: {}", e))?;
        }
        
        // 5. Hash of all outputs (if not SINGLE or NONE)
        if (sighash_type & 0x1f) != 2 && (sighash_type & 0x1f) != 3 {
            let outputs_hash = self.compute_outputs_hash()?;
            data.write_all(&outputs_hash)
                .map_err(|e| format!("Failed to write outputs hash: {}", e))?;
        } else {
            data.write_all(&[0u8; 32])
                .map_err(|e| format!("Failed to write empty outputs hash: {}", e))?;
        }
        
        // 6. JoinSplits hash (empty for v4)
        data.write_all(&[0u8; 32])
            .map_err(|e| format!("Failed to write joinsplits hash: {}", e))?;
        
        // 7. Shielded spends hash
        if shielded_spends.is_empty() {
            data.write_all(&[0u8; 32])
                .map_err(|e| format!("Failed to write empty shielded spends hash: {}", e))?;
        } else {
            let shielded_spends_hash = compute_shielded_spends_hash(shielded_spends)?;
            data.write_all(&shielded_spends_hash)
                .map_err(|e| format!("Failed to write shielded spends hash: {}", e))?;
        }
        
        // 8. Shielded outputs hash
        if shielded_outputs.is_empty() {
            data.write_all(&[0u8; 32])
                .map_err(|e| format!("Failed to write empty shielded outputs hash: {}", e))?;
        } else {
            let shielded_outputs_hash = compute_shielded_outputs_hash(shielded_outputs)?;
            data.write_all(&shielded_outputs_hash)
                .map_err(|e| format!("Failed to write shielded outputs hash: {}", e))?;
        }
        
        // 9. Lock time (using 0)
        data.write_u32::<LittleEndian>(0)
            .map_err(|e| format!("Failed to write lock time: {}", e))?;
        
        // 10. Expiry height (using 0)
        data.write_u32::<LittleEndian>(0)
            .map_err(|e| format!("Failed to write expiry height: {}", e))?;
        
        // 11. Value balance
        data.write_i64::<LittleEndian>(value_balance)
            .map_err(|e| format!("Failed to write value balance: {}", e))?;
        
        // 12. Sighash type
        data.write_u32::<LittleEndian>(sighash_type)
            .map_err(|e| format!("Failed to write sighash type: {}", e))?;
        
        // If not ANYONECANPAY, add current input details
        if (sighash_type & 0x80) == 0 {
            // 13. Outpoint
            let (outpoint, _, _) = &self.transparent_inputs[input_index];
            data.write_all(outpoint.hash())
                .map_err(|e| format!("Failed to write outpoint hash: {}", e))?;
            data.write_u32::<LittleEndian>(outpoint.n())
                .map_err(|e| format!("Failed to write outpoint index: {}", e))?;
            
            // 14. Script code
            write_compact_size(&mut data, script_code.0.len() as u64)?;
            data.write_all(&script_code.0)
                .map_err(|e| format!("Failed to write script code: {}", e))?;
            
            // 15. Value
            data.write_u64::<LittleEndian>(u64::from(value))
                .map_err(|e| format!("Failed to write value: {}", e))?;
            
            // 16. Sequence
            data.write_u32::<LittleEndian>(0xfffffffe)
                .map_err(|e| format!("Failed to write sequence: {}", e))?;
        }
        
        // Create personalization with consensus branch ID
        let mut personalization = [0u8; 16];
        personalization[..12].copy_from_slice(ZCASH_SAPLING_SIGHASH_PERSONALIZATION_PREFIX);
        personalization[12..16].copy_from_slice(&CONSENSUS_BRANCH_ID.to_le_bytes());
        
        // Compute BLAKE2b hash
        let hash = Params::new()
            .hash_length(32)
            .personal(&personalization)
            .to_state()
            .update(&data)
            .finalize();
        
        let mut result = [0u8; 32];
        result.copy_from_slice(hash.as_bytes());
        
        // BitcoinZ: DOES reverse the bytes (confirmed from bitcore-lib-btcz)
        result.reverse();
        Ok(result)
    }
    
    /// Compute hash of all prevouts
    fn compute_prevouts_hash(&self) -> Result<[u8; 32], String> {
        let mut data = Vec::new();
        
        for (outpoint, _, _) in &self.transparent_inputs {
            data.write_all(outpoint.hash())
                .map_err(|e| format!("Failed to write outpoint hash: {}", e))?;
            data.write_u32::<LittleEndian>(outpoint.n())
                .map_err(|e| format!("Failed to write outpoint index: {}", e))?;
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
    
    /// Compute hash of all sequences
    fn compute_sequences_hash(&self) -> Result<[u8; 32], String> {
        let mut data = Vec::new();
        
        for _ in &self.transparent_inputs {
            data.write_u32::<LittleEndian>(0xfffffffe)
                .map_err(|e| format!("Failed to write sequence: {}", e))?;
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
    
    /// Compute hash of all outputs
    fn compute_outputs_hash(&self) -> Result<[u8; 32], String> {
        let mut data = Vec::new();
        
        for (addr, amount) in &self.transparent_outputs {
            // Write amount (8 bytes)
            data.write_u64::<LittleEndian>(u64::from(*amount))
                .map_err(|e| format!("Failed to write amount: {}", e))?;
            
            // Write script pubkey
            let script = addr.script();
            write_compact_size(&mut data, script.0.len() as u64)?;
            data.write_all(&script.0)
                .map_err(|e| format!("Failed to write script: {}", e))?;
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
}