#[macro_use]
extern crate rust_embed;

pub mod bitcoinz_branch;
pub mod bitcoinz_transaction;
pub mod bitcoinz_binding_sig;
pub mod bitcoinz_binding_sig_fix;
pub mod bitcoinz_binding_sig_integration;
pub mod bitcoinz_overwinter;
pub mod bitcoinz_overwinter_builder;
pub mod bitcoinz_legacy_builder;
pub mod bitcoinz_rpc_builder;
pub mod bitcoinz_js_bridge;
pub mod bitcoinz_v4_no_sig;
pub mod bitcoinz_patch;
pub mod bitcoinz_binding_fix;
pub mod bitcoinz_v4_shielded; // Complex API issues with v0.7
// pub mod bitcoinz_shielded_tx;
// pub mod bitcoinz_shielded_sighash;
pub mod bitcoinz_shielded_builder_simple;
pub mod bitcoinz_shielded_simplified;
pub mod bitcoinz_shielded_patch;

// #[cfg(test)]
// mod bitcoinz_shielded_tests;
pub mod blaze;
pub mod commands;
pub mod compact_formats;
pub mod grpc_connector;
pub mod lightclient;
pub mod lightwallet;
pub mod bitcoinz_compat;
pub mod bitcoinz_compat_v2;
pub mod bitcoinz_edwards_bellman;

#[cfg(test)]
mod test_edwards_serialization;

#[cfg(feature = "embed_params")]
#[derive(RustEmbed)]
#[folder = "zcash-params/"]
pub struct SaplingParams;

#[derive(RustEmbed)]
#[folder = "pubkey/"]
pub struct ServerCert;

pub use zcash_primitives::consensus::{MainNetwork, Parameters};

// BitcoinZ Network Implementation
use zcash_primitives::consensus::{self, BlockHeight, NetworkUpgrade, BranchId};
use zcash_address::Network;

#[derive(PartialEq, Copy, Clone, Debug)]
pub struct BitcoinZMainNetwork;

impl Parameters for BitcoinZMainNetwork {
    fn activation_height(&self, nu: NetworkUpgrade) -> Option<BlockHeight> {
        match nu {
            NetworkUpgrade::Overwinter => Some(BlockHeight::from(328500)),
            NetworkUpgrade::Sapling => Some(BlockHeight::from(328500)),
            NetworkUpgrade::Blossom => Some(BlockHeight::from(653600)),
            NetworkUpgrade::Heartwood => Some(BlockHeight::from(903800)),
            NetworkUpgrade::Canopy => Some(BlockHeight::from(1153550)),
            NetworkUpgrade::Nu5 => None, // BitcoinZ doesn't support Nu5 yet
            #[cfg(feature = "zfuture")]
            NetworkUpgrade::ZFuture => None,
        }
    }

    fn coin_type(&self) -> u32 {
        177 // BitcoinZ coin type
    }

    fn hrp_sapling_extended_spending_key(&self) -> &str {
        "secret-extended-key-main"
    }

    fn hrp_sapling_extended_full_viewing_key(&self) -> &str {
        "zxviews"
    }

    fn hrp_sapling_payment_address(&self) -> &str {
        "zs" // BitcoinZ uses same as Zcash for z-addresses
    }

    fn b58_pubkey_address_prefix(&self) -> [u8; 2] {
        [0x1c, 0xb8] // BitcoinZ mainnet pubkey hash prefix
    }

    fn b58_script_address_prefix(&self) -> [u8; 2] {
        [0x1c, 0xbd] // BitcoinZ mainnet script hash prefix
    }

    fn address_network(&self) -> Option<zcash_address::Network> {
        Some(zcash_address::Network::Main)
    }
}

pub const BITCOINZ_MAINNET: BitcoinZMainNetwork = BitcoinZMainNetwork;

// pub mod blaze;
// pub mod compact_formats;
// pub mod grpc_connector;
// pub mod lightclient;
// pub mod lightwallet;

// use lightclient::LightClient;

// fn main() {
//     let seed = std::fs::read_to_string("./testdata/seed.txt").unwrap();
//     let lc = LightClient::new(Some(seed)).unwrap();
//     lc.start_sync();
// }
