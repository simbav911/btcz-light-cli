/// BitcoinZ Shielded Transaction Types
/// 
/// This module provides high-level builders for different types of shielded
/// transactions: t→z, z→t, and z→z

use rand::{CryptoRng, RngCore};

use zcash_primitives::{
    consensus::{BlockHeight, Parameters},
    keys::OutgoingViewingKey,
    legacy::TransparentAddress,
    memo::MemoBytes,
    sapling::{
        keys::ExpandedSpendingKey,
        prover::TxProver,
        Node, Note, PaymentAddress,
    },
    transaction::components::{Amount, transparent},
};

use secp256k1::SecretKey;

use crate::bitcoinz_v4_shielded::{BitcoinZShieldedBuilder, ShieldedSpend, ShieldedOutput};

/// Build a transparent-to-shielded (t→z) transaction
/// This shields transparent funds into the shielded pool
pub fn build_t_to_z_transaction<P, Pr, R>(
    params: &P,
    prover: &Pr,
    height: BlockHeight,
    transparent_inputs: Vec<(transparent::OutPoint, transparent::TxOut, SecretKey)>,
    shielded_outputs: Vec<(OutgoingViewingKey, PaymentAddress, Amount, MemoBytes)>,
    fee: Amount,
    rng: R,
) -> Result<Vec<u8>, String>
where
    P: Parameters,
    Pr: TxProver,
    R: RngCore + CryptoRng + 'static,
{
    let mut builder = BitcoinZShieldedBuilder::new(params.clone(), height, rng);
    
    // Add all transparent inputs
    for (outpoint, coin, key) in transparent_inputs {
        builder.add_transparent_input(outpoint, coin, key)?;
    }
    
    // Add all shielded outputs
    for (ovk, to, value, memo) in shielded_outputs {
        builder.add_sapling_output(ovk, to, value, memo)?;
    }
    
    // Build the transaction
    builder.build(prover, fee)
}

/// Build a shielded-to-transparent (z→t) transaction
/// This unshields funds from the shielded pool to transparent addresses
pub fn build_z_to_t_transaction<P, Pr, R>(
    params: &P,
    prover: &Pr,
    height: BlockHeight,
    shielded_spends: Vec<(ExpandedSpendingKey, Note, Vec<Node>)>,
    transparent_outputs: Vec<(TransparentAddress, Amount)>,
    fee: Amount,
    rng: R,
) -> Result<Vec<u8>, String>
where
    P: Parameters,
    Pr: TxProver,
    R: RngCore + CryptoRng + 'static,
{
    let mut builder = BitcoinZShieldedBuilder::new(params.clone(), height, rng);
    
    // Add all shielded spends
    for (extsk, note, merkle_path) in shielded_spends {
        builder.add_sapling_spend(extsk, note, merkle_path)?;
    }
    
    // Add all transparent outputs
    for (to_addr, value) in transparent_outputs {
        builder.add_transparent_output(to_addr, value)?;
    }
    
    // Build the transaction
    builder.build(prover, fee)
}

/// Build a shielded-to-shielded (z→z) transaction
/// This transfers funds privately within the shielded pool
pub fn build_z_to_z_transaction<P, Pr, R>(
    params: &P,
    prover: &Pr,
    height: BlockHeight,
    shielded_spends: Vec<(ExpandedSpendingKey, Note, Vec<Node>)>,
    shielded_outputs: Vec<(OutgoingViewingKey, PaymentAddress, Amount, MemoBytes)>,
    fee: Amount,
    rng: R,
) -> Result<Vec<u8>, String>
where
    P: Parameters,
    Pr: TxProver,
    R: RngCore + CryptoRng + 'static,
{
    let mut builder = BitcoinZShieldedBuilder::new(params.clone(), height, rng);
    
    // Add all shielded spends
    for (extsk, note, merkle_path) in shielded_spends {
        builder.add_sapling_spend(extsk, note, merkle_path)?;
    }
    
    // Add all shielded outputs
    for (ovk, to, value, memo) in shielded_outputs {
        builder.add_sapling_output(ovk, to, value, memo)?;
    }
    
    // Build the transaction
    builder.build(prover, fee)
}

/// Helper structure for building shielded transactions with change
pub struct ShieldedTransactionBuilder<P: Parameters> {
    params: P,
    height: BlockHeight,
    fee: Amount,
}

impl<P: Parameters> ShieldedTransactionBuilder<P> {
    pub fn new(params: P, height: BlockHeight, fee: Amount) -> Self {
        Self {
            params,
            height,
            fee,
        }
    }
    
    /// Calculate change amount for a transaction
    pub fn calculate_change(
        &self,
        input_total: Amount,
        output_total: Amount,
    ) -> Result<Amount, String> {
        let total_out = (output_total + self.fee)
            .ok_or_else(|| "Output total overflow".to_string())?;
        
        if input_total >= total_out {
            Ok(Amount::from_i64(i64::from(input_total) - i64::from(total_out))
                .map_err(|_| "Change calculation failed".to_string())?)
        } else {
            Err("Insufficient funds".to_string())
        }
    }
    
    /// Build a t→z transaction with automatic change handling
    pub fn build_shield_transaction<Pr, R>(
        &self,
        prover: &Pr,
        transparent_inputs: Vec<(transparent::OutPoint, transparent::TxOut, SecretKey)>,
        shielded_recipient: PaymentAddress,
        amount: Amount,
        memo: MemoBytes,
        ovk: OutgoingViewingKey,
        change_address: Option<TransparentAddress>,
        rng: R,
    ) -> Result<Vec<u8>, String>
    where
        Pr: TxProver,
        R: RngCore + CryptoRng + 'static,
    {
        // Calculate total input
        let input_total = transparent_inputs
            .iter()
            .map(|(_, coin, _)| coin.value)
            .sum::<Amount>();
        
        // Calculate change if needed
        let change = self.calculate_change(input_total, amount)?;
        
        let mut shielded_outputs = vec![(ovk.clone(), shielded_recipient, amount, memo)];
        let mut transparent_outputs = Vec::new();
        
        if change > Amount::zero() {
            if let Some(change_addr) = change_address {
                transparent_outputs.push((change_addr, change));
            } else {
                // Send change to shielded pool as well
                shielded_outputs.push((
                    ovk,
                    shielded_recipient,
                    change,
                    MemoBytes::empty(),
                ));
            }
        }
        
        build_t_to_z_transaction(
            &self.params,
            prover,
            self.height,
            transparent_inputs,
            shielded_outputs,
            self.fee,
            rng,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_change_calculation() {
        // Test change calculation logic
        let params = zcash_primitives::consensus::MainNetwork;
        let builder = ShieldedTransactionBuilder::new(
            params,
            BlockHeight::from_u32(1000),
            Amount::from_u64(1000).unwrap(),
        );
        
        let input = Amount::from_u64(100000).unwrap();
        let output = Amount::from_u64(50000).unwrap();
        
        let change = builder.calculate_change(input, output).unwrap();
        assert_eq!(change, Amount::from_u64(49000).unwrap()); // 100000 - 50000 - 1000
    }
}