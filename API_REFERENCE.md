# API Reference - BitcoinZ Light Wallet

## Core Modules

### `bitcoinz_edwards_bellman`

Edwards point serialization for BitcoinZ's bellman 0.1.0 format.

```rust
pub fn write_edwards_point_bellman<W: io::Write>(
    point: &ExtendedPoint,
    writer: W,
) -> io::Result<()>
```

**Purpose**: Serialize edwards points with BitcoinZ's unique format where the sign bit is stored as bit 63 of the 4th u64.

**Usage**:
```rust
let point = ExtendedPoint::random(&mut rng);
let mut bytes = Vec::new();
write_edwards_point_bellman(&point, &mut bytes)?;
```

### `bitcoinz_v4_shielded`

Main shielded transaction builder for BitcoinZ.

#### Key Functions

```rust
pub fn build_bitcoinz_shielded_transaction<P: Parameters>(
    params: &P,
    height: BlockHeight,
    prover: &LocalTxProver,
    fee: Amount,
    transparent_inputs: Vec<TransparentInputInfo>,
    transparent_outputs: Vec<TxOut>,
    shielded_outputs: Vec<ShieldedOutputInfo>,
) -> Result<Transaction, String>
```

**Purpose**: Build a complete BitcoinZ v4 transaction with shielded components.

**Parameters**:
- `params`: Network parameters (MainNetwork or TestNetwork)
- `height`: Current block height
- `prover`: Zero-knowledge proof generator
- `fee`: Transaction fee in zatoshis
- `transparent_inputs`: List of transparent UTXOs to spend
- `transparent_outputs`: List of transparent outputs
- `shielded_outputs`: List of shielded outputs

### `bitcoinz_overwinter_builder`

Builder for v3 (Overwinter) transparent-only transactions.

```rust
pub fn build_overwinter_tx<P: Parameters>(
    params: &P,
    transparent_inputs: Vec<TransparentInputInfo>,
    transparent_outputs: Vec<TxOut>,
    change_address: Option<TransparentAddress>,
) -> Result<Transaction, String>
```

**Purpose**: Build transparent-only transactions when shielded components aren't needed.

## Transaction Structures

### `TransparentInputInfo`

```rust
pub struct TransparentInputInfo {
    pub utxo: UTXO,
    pub address: TransparentAddress,
    pub secret_key: secp256k1::SecretKey,
}
```

**Fields**:
- `utxo`: The unspent transaction output
- `address`: The transparent address owning the UTXO
- `secret_key`: Private key for signing

### `ShieldedOutputInfo`

```rust
pub struct ShieldedOutputInfo {
    pub address: PaymentAddress,
    pub amount: Amount,
    pub memo: Option<Memo>,
}
```

**Fields**:
- `address`: Recipient's shielded address
- `amount`: Amount in zatoshis
- `memo`: Optional encrypted memo (512 bytes max)

## Cryptographic Functions

### Binding Signature

```rust
fn compute_bitcoinz_binding_signature(
    bsk: &PrivateKey,
    bvk: &PublicKey,
    sighash: &[u8; 32],
) -> Result<Signature, String>
```

**Purpose**: Create BitcoinZ-specific binding signature with 64-byte message.

### Sighash Computation

```rust
pub fn compute_sighash_bitcoinz(
    tx_data: &TransactionData<zcash_primitives::transaction::Authorized>,
) -> [u8; 32]
```

**Purpose**: Compute transaction sighash with BitcoinZ-specific modifications (byte reversal).

## Integration Points

### LightWallet Integration

```rust
use bitcoinzwalletlib::lightwallet::LightWallet;

// Create wallet
let wallet = LightWallet::new(
    config,
    seed_phrase,
    block_height,
)?;

// Send transaction
let txid = wallet.send_transaction(
    recipients,
    fee,
)?;
```

### LightClient Connection

```rust
use bitcoinzwalletlib::lightclient::LightClient;

// Connect to server
let client = LightClient::new(
    "http://93.107.37.216:9067",
    config,
)?;

// Sync blockchain
client.sync().await?;
```

## Constants

```rust
// BitcoinZ consensus parameters
pub const BITCOINZ_VERSION_GROUP_ID: u32 = 0x892f2085;
pub const BITCOINZ_BRANCH_ID: u32 = 0x76b809bb;
pub const DEFAULT_FEE: u64 = 10_000; // 0.0001 BTCZ

// Network configuration
pub const DEFAULT_SERVER: &str = "http://93.107.37.216:9067";
pub const MAX_REORG_SIZE: u64 = 100;
```

## Error Handling

All functions return `Result<T, String>` where errors are descriptive strings.

Common errors:
- `"Insufficient balance"` - Not enough funds
- `"Invalid address"` - Malformed address
- `"Network error"` - Connection issues
- `"Invalid transaction"` - Consensus rule violation

## Usage Examples

### Create and Send Transaction

```rust
// Build transparent transaction
let tx = build_overwinter_tx(
    &BITCOINZ_MAINNET,
    transparent_inputs,
    vec![TxOut {
        value: Amount::from_u64(100_000_000).unwrap(),
        script_pubkey: address.script(),
    }],
    Some(change_address),
)?;

// Broadcast
let txid = client.broadcast_transaction(&tx).await?;
```

### Shield Funds

```rust
// Build shield transaction
let tx = build_bitcoinz_shielded_transaction(
    &BITCOINZ_MAINNET,
    current_height,
    &prover,
    Amount::from_u64(10_000).unwrap(),
    transparent_inputs,
    vec![], // No transparent outputs
    vec![ShieldedOutputInfo {
        address: z_address,
        amount: Amount::from_u64(99_990_000).unwrap(),
        memo: None,
    }],
)?;
```

## Testing Utilities

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_edwards_serialization() {
        let point = ExtendedPoint::random(&mut thread_rng());
        let mut bytes = Vec::new();
        write_edwards_point_bellman(&point, &mut bytes).unwrap();
        assert_eq!(bytes.len(), 32);
    }
}
```

## Future API Additions

Planned additions for full shielded support:
- Note decryption API
- Witness generation
- Merkle tree management
- View key support

## Version Compatibility

- Rust: 1.70+
- zcash_primitives: 0.7.0
- BitcoinZ protocol: Sapling-compatible

## Thread Safety

All public APIs are thread-safe when used with proper synchronization. The wallet maintains internal locks for concurrent access.