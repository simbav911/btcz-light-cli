# Build Instructions - BitcoinZ Light CLI

## Prerequisites

### All Platforms
- Rust 1.70 or later
- Cargo (comes with Rust)
- Git
- Internet connection (for dependencies)

### Platform-Specific Requirements

**macOS**:
- Xcode Command Line Tools
- macOS 10.15 or later

**Linux**:
- gcc or clang
- pkg-config
- OpenSSL development headers

**Windows**:
- Visual Studio 2019 or later with C++ tools
- Windows 10 or later

## Installing Rust

### Using rustup (Recommended)

```bash
# Unix/macOS/Linux
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Windows
# Download and run: https://rustup.rs/
```

### Verify Installation

```bash
rustc --version
cargo --version
```

## Building from Source

### 1. Clone the Repository

```bash
git clone https://github.com/your-repo/btcz-light-cli.git
cd btcz-light-cli
```

### 2. Build the Project

#### Development Build (faster compile, slower runtime)
```bash
cargo build
```

#### Release Build (slower compile, faster runtime)
```bash
cargo build --release
```

### 3. Binary Location

- Development: `target/debug/bitcoinz-light-cli`
- Release: `target/release/bitcoinz-light-cli`

## Platform-Specific Instructions

### macOS

```bash
# Install Xcode Command Line Tools
xcode-select --install

# Clone and build
git clone https://github.com/your-repo/btcz-light-cli.git
cd btcz-light-cli
cargo build --release

# Run
./target/release/bitcoinz-light-cli
```

### Linux (Ubuntu/Debian)

```bash
# Install dependencies
sudo apt update
sudo apt install build-essential pkg-config libssl-dev

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Clone and build
git clone https://github.com/your-repo/btcz-light-cli.git
cd btcz-light-cli
cargo build --release

# Run
./target/release/bitcoinz-light-cli
```

### Linux (Fedora/RHEL/CentOS)

```bash
# Install dependencies
sudo dnf install gcc openssl-devel pkg-config

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Clone and build
git clone https://github.com/your-repo/btcz-light-cli.git
cd btcz-light-cli
cargo build --release

# Run
./target/release/bitcoinz-light-cli
```

### Windows

1. Install Visual Studio 2019 or later with C++ tools
2. Install Rust from https://rustup.rs/
3. Open "x64 Native Tools Command Prompt for VS"

```cmd
# Clone and build
git clone https://github.com/your-repo/btcz-light-cli.git
cd btcz-light-cli
cargo build --release

# Run
target\release\bitcoinz-light-cli.exe
```

## Build Options

### Feature Flags

```bash
# Build without default features
cargo build --release --no-default-features

# Build with specific features
cargo build --release --features "feature1,feature2"
```

### Cross-Compilation

```bash
# Add target
rustup target add x86_64-pc-windows-gnu

# Build for target
cargo build --release --target x86_64-pc-windows-gnu
```

## Troubleshooting Build Issues

### Common Problems

**1. "error: linker `cc` not found"**
- Install build tools for your platform
- Linux: `sudo apt install build-essential`
- macOS: `xcode-select --install`

**2. OpenSSL errors**
- Linux: Install `libssl-dev` or `openssl-devel`
- macOS: OpenSSL should be available via Homebrew
- Windows: Usually bundled, may need Visual Studio

**3. Out of memory during build**
```bash
# Limit parallel jobs
cargo build -j 2 --release
```

**4. Old Rust version**
```bash
# Update Rust
rustup update
```

### Clean Build

If you encounter issues:

```bash
# Clean build artifacts
cargo clean

# Rebuild
cargo build --release
```

## Optimizations

### Release Build Optimizations

Edit `Cargo.toml`:
```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
```

### Strip Binary (Reduce Size)

```bash
# Linux/macOS
strip target/release/bitcoinz-light-cli

# Check size
ls -lh target/release/bitcoinz-light-cli
```

## Testing the Build

### Run Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

### Verify Binary

```bash
# Check version
./target/release/bitcoinz-light-cli --version

# Test connection
./target/release/bitcoinz-light-cli --server http://93.107.37.216:9067
```

## Creating Distribution Package

### Linux/macOS

```bash
# Create tarball
tar -czf bitcoinz-light-cli-linux-x64.tar.gz -C target/release bitcoinz-light-cli

# Create with version
VERSION=$(cargo pkgid | cut -d# -f2)
tar -czf bitcoinz-light-cli-${VERSION}-linux-x64.tar.gz -C target/release bitcoinz-light-cli
```

### Windows

```powershell
# Create zip
Compress-Archive -Path target\release\bitcoinz-light-cli.exe -DestinationPath bitcoinz-light-cli-windows-x64.zip
```

## Development Setup

### IDE Setup

**VS Code**:
1. Install rust-analyzer extension
2. Install CodeLLDB for debugging

**IntelliJ/CLion**:
1. Install Rust plugin
2. Import as Cargo project

### Running in Development

```bash
# Run directly with cargo
cargo run

# Run with arguments
cargo run -- --server http://localhost:9067

# Run with debug output
RUST_LOG=debug cargo run
```

## Contributing

1. Fork the repository
2. Create feature branch
3. Make changes
4. Run tests
5. Submit pull request

See CONTRIBUTING.md for details.