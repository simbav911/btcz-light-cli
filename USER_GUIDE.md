# BitcoinZ Light CLI User Guide

## Table of Contents
1. [Getting Started](#getting-started)
2. [Basic Commands](#basic-commands)
3. [Managing Addresses](#managing-addresses)
4. [Sending Transactions](#sending-transactions)
5. [Shielding Funds](#shielding-funds)
6. [Advanced Features](#advanced-features)
7. [Troubleshooting](#troubleshooting)
8. [Security Best Practices](#security-best-practices)

## Getting Started

### First Run

When you run the wallet for the first time, it will:
1. Create a new wallet with a 24-word seed phrase
2. Generate addresses (both transparent and shielded)
3. Connect to the lightwalletd server
4. Start synchronizing with the blockchain

```bash
./target/release/bitcoinz-light-cli
```

You'll see output like:
```
Lightclient connecting to http://93.107.37.216:9067/
{
  "result": "success",
  "latest_block": 1577275,
  "total_blocks_synced": 1122
}
Ready!
```

### Backup Your Seed Phrase

**IMPORTANT**: Immediately backup your seed phrase!

```bash
seed
```

Write down these 24 words in order. This is the ONLY way to recover your wallet if something happens to your device.

## Basic Commands

### Check Your Balance

```bash
balance
```

Output shows:
- `tbalance` - Transparent balance (publicly visible)
- `zbalance` - Shielded balance (private)
- Individual address balances

### View Your Addresses

```bash
addresses
```

You'll see:
- `t_addresses` - Transparent addresses (start with 't1')
- `z_addresses` - Shielded addresses (start with 'zs1')

### Transaction History

```bash
list
```

Shows all your transactions with:
- Block height
- Transaction ID
- Amount (negative for outgoing)
- Date/time

### Get Help

```bash
help
```

Lists all available commands with descriptions.

## Managing Addresses

### Create New Address

```bash
# New transparent address
new t

# New shielded address
new z
```

### Export Private Keys

**WARNING**: Keep private keys secret!

```bash
export
```

## Sending Transactions

### Send Transparent Transaction

Amount is in zatoshis (1 BTCZ = 100,000,000 zatoshis):

```bash
# Send 1 BTCZ
send t1bzjjWe5gD28AcCW36FVV3t76XnzdAyguw 100000000

# Send 0.5 BTCZ
send t1bzjjWe5gD28AcCW36FVV3t76XnzdAyguw 50000000
```

### Send with Multiple Recipients

```bash
send '[{"address": "t1addr1...", "amount": 100000000}, {"address": "t1addr2...", "amount": 50000000}]'
```

### Transaction Fees

Default fee: 0.0001 BTCZ (10,000 zatoshis)

## Shielding Funds

### Shield Transparent Funds (t→z)

To shield your transparent funds for privacy:

```bash
# Shield amount (in BTCZ) from t-address to z-address
shield <from_taddr> <to_zaddr> <amount> <fee>

# Example: Shield 1.5 BTCZ with 0.001 BTCZ fee
shield t1dWCXCaMn2tJqUuzxTPRNXfmaLQQVnYPcN zs1k3wanq50ae50lgujv9jkh0p2lq5wn99u8l0j5d4q8tmssv9krrpzcry4xs3jtsceg38qz9ctpnn 1.5 0.001
```

### Check Shielding Progress

```bash
sendprogress
```

## Advanced Features

### Rescan Blockchain

If transactions are missing:

```bash
rescan
```

### Connect to Different Server

```bash
# At startup
./target/release/bitcoinz-light-cli --server http://your-server:9067
```

### Import Wallet

```bash
# From seed phrase
./target/release/bitcoinz-light-cli --seed "your 24 word seed phrase"

# With birthday height (faster sync)
./target/release/bitcoinz-light-cli --seed "your 24 word seed phrase" --birthday 500000
```

### Wallet Encryption

```bash
# Encrypt wallet
encrypt "your-strong-password"

# Unlock encrypted wallet
unlock "your-password"

# Lock wallet
lock
```

## Troubleshooting

### Common Issues

**1. Connection Failed**
- Check internet connection
- Verify server is running
- Try default server: `93.107.37.216:9067`

**2. Balance Not Showing**
- Wait for sync to complete
- Run `rescan` command
- Check `height` matches latest block

**3. Transaction Not Confirmed**
- Check transaction fee (minimum 0.0001 BTCZ)
- Wait for next block (~2.5 minutes)
- Use `list` to check status

**4. "Invalid address" Error**
- Verify address format (t1... or zs1...)
- Check for typos
- Ensure address is for BitcoinZ (not Zcash)

### Debug Mode

For detailed logs:
```bash
./target/release/bitcoinz-light-cli --debug
```

Check log file:
- macOS: `~/Library/Application Support/BitcoinZ/bitcoinz-wallet.debug.log`
- Linux: `~/.bitcoinz/bitcoinz-wallet.debug.log`

## Security Best Practices

### 1. Seed Phrase Security
- Write it down on paper (not digital)
- Store in secure location
- Never share with anyone
- Test recovery before storing large amounts

### 2. Address Usage
- Use new addresses for each transaction
- Use z-addresses for privacy
- Don't reuse addresses

### 3. Wallet Security
- Use strong encryption password
- Lock wallet when not in use
- Keep software updated
- Verify server certificates

### 4. Transaction Privacy
- Shield funds before sending privately
- Use z→z transactions for full privacy
- Be aware t-addresses are public

### 5. Backup Strategy
- Backup seed phrase
- Test wallet recovery
- Keep multiple secure copies
- Update after creating new addresses

## Amount Conversions

| BTCZ | Zatoshis | Scientific |
|------|----------|------------|
| 1 | 100,000,000 | 1e8 |
| 0.1 | 10,000,000 | 1e7 |
| 0.01 | 1,000,000 | 1e6 |
| 0.001 | 100,000 | 1e5 |
| 0.0001 | 10,000 | 1e4 |

## Getting Help

- Use `help` command in wallet
- Check error messages carefully
- Review this guide
- Submit issues on GitHub
- Join BitcoinZ community