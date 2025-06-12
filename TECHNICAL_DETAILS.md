# Technical Details - BitcoinZ Light Wallet

## Architecture Overview

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  BitcoinZ Light │────▶│   lightwalletd   │────▶│  BitcoinZ Node  │
│       CLI       │     │    (Bridge)      │     │   (Full Node)   │
└─────────────────┘     └──────────────────┘     └─────────────────┘
     Your Wallet         Port 9067              Port 1979
    (Private Keys)      (No Private Data)      (Full Blockchain)
```

The wallet uses a three-tier architecture:
1. **Client (this wallet)** - Holds private keys, creates transactions
2. **lightwalletd** - Bridge server that indexes blockchain data
3. **BitcoinZ Full Node** - Maintains complete blockchain

## BitcoinZ Protocol Differences from Zcash

BitcoinZ forked from Zcash but has several critical protocol differences that required custom implementations:

### 1. Edwards Point Serialization

**Problem**: BitcoinZ uses bellman 0.1.0 library which has a different edwards point serialization format.

**Zcash (modern)**:
- Stores sign bit as bit 255 (MSB of last byte)
- Uses standard byte array representation

**BitcoinZ (bellman 0.1.0)**:
- Treats field representation as array of u64s
- Stores sign bit as bit 63 of the 4th u64
- Requires special handling for little-endian conversion

**Solution** (`lib/src/bitcoinz_edwards_bellman.rs`):
```rust
// BitcoinZ bellman 0.1.0 format
if x_is_odd {
    let y_bytes = y_repr.as_mut();
    // The 4th u64 starts at byte 24 in little-endian
    let mut high_u64 = u64::from_le_bytes([
        y_bytes[24], y_bytes[25], y_bytes[26], y_bytes[27],
        y_bytes[28], y_bytes[29], y_bytes[30], y_bytes[31]
    ]);
    // Set bit 63 of the 4th u64
    high_u64 |= 0x8000000000000000u64;
    let after_bytes = high_u64.to_le_bytes();
    y_bytes[24..32].copy_from_slice(&after_bytes);
}
```

### 2. Binding Signature Algorithm

**Problem**: BitcoinZ uses a different binding signature message format.

**Zcash**:
```
message = sighash (32 bytes)
signature = sign(bsk, message)
```

**BitcoinZ**:
```
message = bvk || sighash (64 bytes)
signature = sign(bsk, message)
```

**Solution** (`lib/src/bitcoinz_v4_shielded.rs`):
```rust
fn compute_bitcoinz_binding_signature(
    bsk: &PrivateKey,
    bvk: &PublicKey,
    sighash: &[u8; 32],
) -> Result<Signature, String> {
    let mut message = [0u8; 64];
    // Serialize bvk (first 32 bytes)
    let mut bvk_bytes = [0u8; 32];
    bvk.write(&mut bvk_bytes[..]).map_err(|e| e.to_string())?;
    message[..32].copy_from_slice(&bvk_bytes);
    // Add sighash (last 32 bytes)
    message[32..].copy_from_slice(sighash);
    
    // Sign the 64-byte message
    let sig = bsk.sign(&message, &mut rng, BINDING_SIG_HASH_PERSONALIZATION, &JUBJUB);
    Ok(sig)
}
```

### 3. Transaction Format Differences

**Fixed Parameters**:
- Version Group ID: `0x892f2085` (always)
- Consensus Branch ID: `1991772603` (0x76b809bb) (always Sapling)
- No expiry height (always 0)

**Sighash Computation**:
- BitcoinZ does NOT reverse bytes after BLAKE2b computation
- Zcash reverses the final hash bytes

## Key Implementation Files

### Core BitcoinZ Implementations

1. **`lib/src/bitcoinz_edwards_bellman.rs`**
   - Implements exact edwards point serialization for bellman 0.1.0
   - Critical for transaction validation

2. **`lib/src/bitcoinz_v4_shielded.rs`**
   - Main shielded transaction builder
   - Implements custom binding signature
   - Handles all shielded transaction logic

3. **`lib/src/bitcoinz_overwinter_builder.rs`**
   - Builds v3 (Overwinter) transparent-only transactions
   - Fallback for when v4 isn't needed

4. **`lib/src/bitcoinz_binding_sig_fix.rs`**
   - Various binding signature implementations
   - Compatibility layers for different scenarios

### Integration Files

1. **`lib/src/lightwallet.rs`**
   - Main wallet logic integration
   - Transaction creation and broadcasting

2. **`lib/src/lightclient.rs`**
   - gRPC client for lightwalletd communication
   - Handles blockchain synchronization

## Transaction Building Process

### Transparent Transactions (t→t)

1. Select UTXOs for inputs
2. Create outputs with amounts
3. Calculate fee
4. Build v4 transaction without Sapling bundle
5. Sign with SIGHASH_ALL
6. Broadcast to network

### Shielded Transactions (t→z)

1. Select transparent inputs
2. Create shielded outputs using Sapling circuit
3. Generate zero-knowledge proofs
4. Calculate value balance (negative for t→z)
5. Create binding signature with BitcoinZ algorithm
6. Serialize with proper edwards point format
7. Broadcast to network

## API Limitations and Workarounds

### zcash_primitives v0.7.0 Limitations

The library doesn't expose necessary internals for BitcoinZ compatibility:
- Cannot access raw transaction builder components
- Cannot customize sighash computation
- Cannot modify edwards point serialization

### Workarounds Implemented

1. **Custom Transaction Builders** - Reimplemented transaction building from scratch
2. **Direct Serialization** - Bypass library serialization for custom formats
3. **Monkey Patching** - Post-process transactions to fix BitcoinZ-specific issues

## Cryptographic Parameters

```rust
// BitcoinZ consensus parameters
const BITCOINZ_VERSION_GROUP_ID: u32 = 0x892f2085;
const BITCOINZ_BRANCH_ID: u32 = 0x76b809bb; // Sapling
const BINDING_SIG_HASH_PERSONALIZATION: &[u8; 16] = b"ZcashBindingSig";
```

## Testing Infrastructure

The implementation includes comprehensive testing:
- Unit tests for edwards point serialization
- Integration tests for transaction building
- Mainnet verification of all transaction types

## Performance Considerations

- Initial sync: ~5-10 minutes (vs days for full node)
- Transaction creation: <1 second
- Memory usage: <100MB
- Network bandwidth: Minimal (only block headers + relevant transactions)

## Security Model

1. **Private keys** - Never leave the client device
2. **View keys** - Not shared with lightwalletd
3. **Transaction privacy** - Shielded transactions hide amounts and recipients
4. **Network privacy** - Consider using Tor for IP privacy

## Future Improvements

1. Complete z→z and z→t transaction support
2. Hardware wallet integration
3. Mobile wallet libraries
4. Performance optimizations
5. Fork zcash_primitives for better BitcoinZ support