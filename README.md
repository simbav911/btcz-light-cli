# BitcoinZ Light CLI

A lightweight command-line BitcoinZ wallet that uses the lightwalletd protocol for fast synchronization without downloading the entire blockchain.

## Overview

BitcoinZ Light CLI is a privacy-preserving wallet that connects to a lightwalletd server to provide:
- Fast blockchain synchronization (minutes instead of days)
- Low storage requirements (< 100MB vs 50GB+ for full node)
- Support for both transparent (t) and shielded (z) addresses
- Complete transaction privacy with z-addresses

## Current Implementation Status

| Transaction Type | Status | Notes |
|-----------------|---------|-------|
| Transparent → Transparent (t→t) | ✅ **Working** | Fully functional on mainnet |
| Transparent → Shielded (t→z) | ✅ **Working** | Shield your transparent funds |
| Shielded → Shielded (z→z) | 🚧 Testing | Infrastructure complete |
| Shielded → Transparent (z→t) | 🚧 Testing | Infrastructure complete |

## Features

- **Fast sync** - Uses lightwalletd protocol for efficient blockchain synchronization
- **Full privacy** - Supports shielded (z-addresses) and transparent addresses
- **HD wallet** - Hierarchical deterministic wallet with seed phrase backup
- **Multi-platform** - Works on Windows, macOS, and Linux
- **Secure** - Private keys never leave your device

## Installation

### Build from Source

Requirements:
- Rust 1.70 or later
- Cargo

```bash
git clone https://github.com/your-repo/btcz-light-cli
cd btcz-light-cli
cargo build --release
```

The binary will be in `target/release/bitcoinz-light-cli`

### Pre-built Binaries

Coming soon - pre-built binaries for major platforms.

## Usage

### Basic Usage

Start the wallet (connects to default server at 93.107.37.216:9067):
```bash
./target/release/bitcoinz-light-cli
```

### Connect to a Custom Server

```bash
./target/release/bitcoinz-light-cli --server http://your-server:9067
```

### Common Commands

Once the wallet is running, you can use these commands:

#### Basic Commands
- `help` - Show all available commands
- `addresses` - List all wallet addresses  
- `balance` - Show current balance
- `quit` - Exit the wallet

#### Transaction Commands
- `send <address> <amount_in_zatoshis>` - Send transparent transaction
- `shield <from_taddr> <to_zaddr> <amount> <fee>` - Shield transparent funds
- `list` - Show transaction history

#### Example Commands
```bash
# Check balance
balance

# Get your addresses
addresses

# Send transparent transaction (amount in zatoshis, 1 BTCZ = 100,000,000 zatoshis)
send t1bzjjWe5gD28AcCW36FVV3t76XnzdAyguw 100000000

# Shield funds (amounts in BTCZ)
shield t1dWCXCaMn2tJqUuzxTPRNXfmaLQQVnYPcN zs1k3wanq50ae50lgujv9jkh0p2lq5wn99u8l0j5d4q8tmssv9krrpzcry4xs3jtsceg38qz9ctpnn 1.5 0.001

# View transaction history
list
```

### Restore from Seed

```bash
./target/release/bitcoinz-light-cli --seed "your 24 word seed phrase" --birthday 328500
```

The birthday is the block height when your wallet was created. Use 0 to scan from the beginning.

## Technical Implementation

This wallet implements significant BitcoinZ-specific modifications:

### Key Technical Achievements
1. **Edwards Point Serialization** - Custom implementation for BitcoinZ's bellman 0.1.0 format
2. **Binding Signature Algorithm** - BitcoinZ uses `sign(bsk, bvk || sighash)` vs Zcash's `sign(bsk, sighash)`
3. **Transaction Builders** - Custom builders to handle BitcoinZ's unique requirements

See [TECHNICAL_DETAILS.md](TECHNICAL_DETAILS.md) for in-depth technical information.

## Configuration

The wallet stores its data in:
- macOS: `~/Library/Application Support/BitcoinZ/`
- Linux: `~/.bitcoinz/`
- Windows: `%APPDATA%\BitcoinZ\`

Files:
- `bitcoinz-wallet.dat` - Encrypted wallet file
- `bitcoinz-wallet.debug.log` - Debug log file

## Security

- **Backup your seed phrase** - This is the only way to recover your wallet
- **Private keys stay local** - Your keys never leave your device
- **Server privacy** - The lightwalletd server cannot see your private keys or shielded transaction details

## Development Journey

This implementation required solving several challenging technical problems due to BitcoinZ's protocol differences from Zcash. The most significant challenges were:

1. Discovering BitcoinZ's unique edwards point serialization format
2. Implementing the custom binding signature algorithm
3. Working around zcash_primitives API limitations

See [CHALLENGES_AND_SOLUTIONS.md](CHALLENGES_AND_SOLUTIONS.md) for the full development story.

## Building and Testing

See [BUILD_INSTRUCTIONS.md](BUILD_INSTRUCTIONS.md) for detailed build instructions and [TESTING_RESULTS.md](TESTING_RESULTS.md) for test results.

## License

MIT License - see [LICENSE](LICENSE) file for details

## Credits

Based on Zecwallet Light CLI by Aditya Kulkarni, extensively modified for BitcoinZ protocol compatibility.

## Support

- GitHub Issues: [Create an issue](https://github.com/your-repo/btcz-light-cli/issues)
- BitcoinZ Community: [bitcoinz.global](https://bitcoinz.global)