# BitcoinZ Light Client Implementation Status

## Overview
This document describes the current implementation status of the BitcoinZ light wallet client and the challenges with shielded transactions.

## Working Features ✅

### 1. Transparent-to-Transparent (t→t) Transactions
- **Status**: Fully functional
- **Transaction ID**: 7ca9d30f60cf1e9d3be0cfefbaeccd713cf04b631722d5882e6ce9373cde6065
- **Implementation**: Uses custom v4 transaction builder without binding signatures
- **Key Discoveries**:
  - BitcoinZ uses fixed consensus branch ID: 1991772603 (0x76b809bb)
  - BitcoinZ does NOT reverse sighash bytes after BLAKE2b computation
  - Version group ID: 0x892f2085
  - Expiry height should be 0

## Not Yet Implemented ❌

### 2. Shielded Transactions (t→z, z→t, z→z)
- **Status**: Not supported
- **Error**: "bad-txns-sapling-binding-signature-invalid"
- **Root Cause**: Incompatible binding signature formats

#### Technical Details

**Binding Signature Difference:**
- **Zcash**: `sign(bsk, sighash)` - Signs only the 32-byte sighash
- **BitcoinZ**: `sign(bsk, bvk || sighash)` - Signs a 64-byte message

Where:
- `bsk` = binding signing key (accumulated from value commitments)
- `bvk` = binding verification key (bsk * G on Jubjub curve)
- `sighash` = transaction hash
- `||` = concatenation

#### Implementation Challenges

1. **API Incompatibility**: The zcash_primitives library (v0.7.0) uses a sealed transaction builder that doesn't expose the internal cryptographic values needed (bsk, bvk).

2. **Version Mismatch**: Creating a custom builder requires specific versions of cryptographic libraries (jubjub, bellman, etc.) that have different APIs than what we're using.

3. **Complexity**: Implementing shielded transactions requires:
   - Sapling spend proofs
   - Sapling output proofs
   - Note encryption/decryption
   - Merkle path computation
   - Value commitment accumulation
   - Proper key derivation

## Recommendations

### For Users
- **Use transparent addresses only** for BitcoinZ transactions
- Transparent addresses start with 't1' (e.g., t1gyheDB7BYCfbPJitCLV28518AKHWeF9Ra)
- Shielded addresses start with 'zs' but are not currently supported

### For Developers
To implement shielded transactions, one would need to:

1. **Fork zcash_primitives**: Modify the transaction builder to expose bsk/bvk or allow custom binding signature computation

2. **Create Custom Builder**: Build transactions from scratch with BitcoinZ's binding signature format

3. **Update Dependencies**: Ensure all cryptographic libraries are compatible versions

4. **Extensive Testing**: Test all shielded transaction types on testnet before mainnet

## Current Wallet Behavior

When attempting shielded transactions, the wallet will:
1. Detect the transaction type
2. Display an informative error message
3. Suggest using transparent addresses instead

Example:
```
BitcoinZ: Shielded transactions are not yet fully implemented
BitcoinZ: The binding signature format is incompatible
Error: BitcoinZ shielded transactions not yet supported. Use transparent addresses for now.
```

## Conclusion

The BitcoinZ light wallet client successfully handles transparent transactions but cannot process shielded transactions due to fundamental differences in the binding signature algorithm. This is a protocol-level incompatibility that requires significant development effort to resolve.