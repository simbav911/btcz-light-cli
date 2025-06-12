# BitcoinZ Shielded Transaction Implementation Plan

## Overview

This document details the implementation plan for enabling shielded transactions (t→z, z→t, z→z) in the BitcoinZ light client. Based on our successful implementation of transparent transactions, we now understand the key differences between BitcoinZ and Zcash that need to be addressed.

## Key Technical Challenges

### 1. Binding Signature Algorithm
The primary challenge is that BitcoinZ computes binding signatures differently than Zcash:

- **Zcash**: `sign(bsk, sighash)` - Signs only the 32-byte sighash
- **BitcoinZ**: `sign(bsk, bvk || sighash)` - Signs a 64-byte message (binding verification key + sighash)

### 2. Transaction Structure for Shielded
For shielded transactions, we need to handle:
- Sapling spends (shielded inputs)
- Sapling outputs (shielded outputs)
- Value balance (net value flow between transparent and shielded pools)
- Binding signature (REQUIRED for any transaction with shielded components)

## Implementation Strategy

### Phase 2.1: Core Shielded Transaction Builder

#### File: `lib/src/bitcoinz_v4_shielded.rs`

```rust
/// BitcoinZ v4 Shielded Transaction Builder
/// 
/// Builds v4 Sapling transactions with proper binding signatures
/// for shielded transfers (t→z, z→t, z→z)

use zcash_primitives::{
    sapling::{
        redjubjub::{PrivateKey, PublicKey, Signature},
        spend_sig, output_sig,
        Note, PaymentAddress,
    },
    transaction::components::sapling::{
        SpendDescription, OutputDescription,
    },
};

pub struct BitcoinZShieldedBuilder {
    // Transaction components
    transparent_inputs: Vec<TransparentInput>,
    transparent_outputs: Vec<TransparentOutput>,
    sapling_spends: Vec<SaplingSpend>,
    sapling_outputs: Vec<SaplingOutput>,
    
    // Binding signature key (accumulator)
    binding_sig_key: PrivateKey,
}

impl BitcoinZShieldedBuilder {
    pub fn new() -> Self {
        // Initialize with random binding signature key
    }
    
    pub fn add_sapling_spend(&mut self, spend: SaplingSpend) {
        // Add spend and update binding signature key
    }
    
    pub fn add_sapling_output(&mut self, output: SaplingOutput) {
        // Add output and update binding signature key
    }
    
    pub fn build(self) -> Result<Vec<u8>, Error> {
        // Build complete transaction with BitcoinZ binding signature
    }
}
```

### Phase 2.2: BitcoinZ Binding Signature Implementation

#### Key Algorithm:
```rust
fn compute_bitcoinz_binding_signature(
    bsk: &PrivateKey,
    bvk: &PublicKey,
    sighash: &[u8; 32],
) -> Signature {
    // Create 64-byte message: bvk || sighash
    let mut message = [0u8; 64];
    message[..32].copy_from_slice(&bvk.to_bytes());
    message[32..].copy_from_slice(sighash);
    
    // Sign with RedJubjub
    bsk.sign(&message)
}
```

### Phase 2.3: Transaction Type Implementations

#### 1. Transparent-to-Shielded (t→z)
```rust
pub fn build_t_to_z_transaction(
    transparent_inputs: Vec<(OutPoint, TxOut, SecretKey)>,
    shielded_outputs: Vec<(PaymentAddress, Amount, Memo)>,
) -> Result<Vec<u8>, Error> {
    let mut builder = BitcoinZShieldedBuilder::new();
    
    // Add transparent inputs
    for input in transparent_inputs {
        builder.add_transparent_input(input);
    }
    
    // Add shielded outputs
    for (address, amount, memo) in shielded_outputs {
        builder.add_sapling_output(address, amount, memo)?;
    }
    
    builder.build()
}
```

#### 2. Shielded-to-Transparent (z→t)
```rust
pub fn build_z_to_t_transaction(
    shielded_inputs: Vec<SpendInfo>,
    transparent_outputs: Vec<(TransparentAddress, Amount)>,
) -> Result<Vec<u8>, Error> {
    let mut builder = BitcoinZShieldedBuilder::new();
    
    // Add shielded spends
    for spend_info in shielded_inputs {
        builder.add_sapling_spend(spend_info)?;
    }
    
    // Add transparent outputs
    for (address, amount) in transparent_outputs {
        builder.add_transparent_output(address, amount);
    }
    
    builder.build()
}
```

#### 3. Shielded-to-Shielded (z→z)
```rust
pub fn build_z_to_z_transaction(
    shielded_inputs: Vec<SpendInfo>,
    shielded_outputs: Vec<(PaymentAddress, Amount, Memo)>,
) -> Result<Vec<u8>, Error> {
    let mut builder = BitcoinZShieldedBuilder::new();
    
    // Add shielded spends
    for spend_info in shielded_inputs {
        builder.add_sapling_spend(spend_info)?;
    }
    
    // Add shielded outputs
    for (address, amount, memo) in shielded_outputs {
        builder.add_sapling_output(address, amount, memo)?;
    }
    
    builder.build()
}
```

### Phase 2.4: Integration with LightWallet

#### Update `lightwallet.rs`:
```rust
// In send_to_address_internal
match detect_tx_type(...) {
    BitcoinZTxType::TransparentToTransparent => {
        // Use existing v4_no_sig builder
    },
    BitcoinZTxType::TransparentToShielded => {
        // Use new t→z builder
        build_t_to_z_transaction(...)
    },
    BitcoinZTxType::ShieldedToTransparent => {
        // Use new z→t builder
        build_z_to_t_transaction(...)
    },
    BitcoinZTxType::ShieldedToShielded => {
        // Use new z→z builder
        build_z_to_z_transaction(...)
    },
}
```

## Technical Details

### 1. Sapling Spend Structure
```rust
struct SpendDescription {
    cv: jubjub::ExtendedPoint,           // Value commitment
    anchor: bls12_381::Scalar,           // Merkle tree root
    nullifier: [u8; 32],                 // Spend nullifier
    rk: PublicKey,                       // Randomized public key
    zkproof: [u8; GROTH_PROOF_SIZE],     // Zero-knowledge proof
    spend_auth_sig: Signature,           // Spend authorization signature
}
```

### 2. Sapling Output Structure
```rust
struct OutputDescription {
    cv: jubjub::ExtendedPoint,           // Value commitment
    cm: bls12_381::Scalar,               // Note commitment
    ephemeral_key: jubjub::ExtendedPoint, // For note encryption
    enc_ciphertext: [u8; 580],           // Encrypted note
    out_ciphertext: [u8; 80],            // Encrypted memo key
    zkproof: [u8; GROTH_PROOF_SIZE],     // Zero-knowledge proof
}
```

### 3. Sighash Computation for Shielded
When shielded components are present:
- `hashShieldedSpends` = BLAKE2b-256 of all spend descriptions
- `hashShieldedOutputs` = BLAKE2b-256 of all output descriptions
- `valueBalance` = net value flow (can be positive or negative)

### 4. Critical BitcoinZ Constants
```rust
// Same as transparent transactions
const CONSENSUS_BRANCH_ID: u32 = 1991772603;  // 0x76b809bb
const VERSION_GROUP_ID: u32 = 0x892f2085;
const SAPLING_TX_VERSION: i32 = 4;

// Shielded-specific
const GROTH_PROOF_SIZE: usize = 192;
const SAPLING_SPEND_SIZE: usize = 384;
const SAPLING_OUTPUT_SIZE: usize = 948;
```

## Implementation Steps

1. **Create base shielded builder** (bitcoinz_v4_shielded.rs)
   - Transaction structure with shielded components
   - Binding signature key management
   - Value balance calculation

2. **Implement BitcoinZ binding signature**
   - 64-byte message format (bvk || sighash)
   - RedJubjub signing with BitcoinZ algorithm
   - Signature serialization

3. **Add Sapling spend support**
   - Spend description creation
   - Nullifier computation
   - Spend authorization signatures
   - Zero-knowledge proof integration

4. **Add Sapling output support**
   - Output description creation
   - Note encryption
   - Memo handling
   - Zero-knowledge proof integration

5. **Implement transaction type builders**
   - t→z: Transparent inputs, shielded outputs
   - z→t: Shielded inputs, transparent outputs
   - z→z: Shielded inputs and outputs

6. **Integration and testing**
   - Update wallet integration
   - Add comprehensive tests
   - Test on BitcoinZ testnet

## Testing Plan

### 1. Unit Tests
- Test binding signature computation
- Test transaction serialization
- Test sighash computation with shielded components

### 2. Integration Tests
- Test each transaction type (t→z, z→t, z→z)
- Test with various input/output combinations
- Test error conditions

### 3. Network Tests
- Test on BitcoinZ testnet first
- Verify transactions are accepted
- Test with mainnet once stable

## Success Criteria

1. ✅ All shielded transaction types build successfully
2. ✅ Binding signatures validate on BitcoinZ network
3. ✅ Transactions are accepted and confirmed
4. ✅ Funds move correctly between transparent and shielded pools
5. ✅ All tests pass consistently

## Risks and Mitigations

1. **Risk**: Binding signature algorithm mismatch
   - **Mitigation**: Extensive testing and comparison with BitcoinZ core

2. **Risk**: Zero-knowledge proof generation issues
   - **Mitigation**: Use proven Sapling parameters and libraries

3. **Risk**: Note encryption incompatibility
   - **Mitigation**: Follow Zcash note encryption exactly (likely unchanged)

## Timeline Estimate

- Week 1: Core shielded builder and binding signature
- Week 2: Sapling spend/output implementation
- Week 3: Transaction type builders and integration
- Week 4: Testing and debugging
- Week 5: Documentation and mainnet testing

Total: ~5 weeks for complete shielded transaction support