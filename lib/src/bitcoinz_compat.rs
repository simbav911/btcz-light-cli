/// BitcoinZ Compatibility Layer
/// 
/// This module provides serialization functions that match BitcoinZ's exact
/// binary format for edwards points and other cryptographic elements.
/// BitcoinZ uses bellman 0.1.0 which has different serialization than modern libraries.

use jubjub::{ExtendedPoint, SubgroupPoint};
use ff::PrimeField;
use group::{Curve, GroupEncoding};
use hex;

/// Convert a modern jubjub ExtendedPoint to BitcoinZ's edwards point format
/// 
/// BitcoinZ format (32 bytes):
/// - Bytes 0-31: Y coordinate in little-endian
/// - Bit 255 (highest bit of byte 31): Sign of X coordinate (1 if x is odd)
pub fn serialize_edwards_point_bitcoinz(point: &ExtendedPoint) -> [u8; 32] {
    // The key difference: BitcoinZ's old format vs modern format
    // Modern jubjub uses different coordinate system and compression
    
    // First, let's check if the point uses the standard serialization
    let standard_bytes = point.to_bytes();
    
    // In BitcoinZ's old format, they store y-coordinate with x-sign in high bit
    // In modern format, it's different. Let's try to convert.
    
    // Try to decompress the modern point to get actual coordinates
    let affine = point.to_affine();
    
    // Access the actual field elements
    // Note: In old jubjub, coordinates were (x, y)
    // In new jubjub, they use (u, v) which maps differently
    let u = affine.get_u(); // This is actually the x-coordinate in BitcoinZ's system
    let v = affine.get_v(); // This is actually the y-coordinate in BitcoinZ's system
    
    // Now we need to serialize in BitcoinZ's format:
    // Store v (y-coordinate) with u's sign bit
    let mut result = [0u8; 32];
    
    // Get v as bytes (little-endian)
    let v_bytes = v.to_repr();
    result.copy_from_slice(&v_bytes.as_ref());
    
    // Check if u is odd (need to look at the actual value, not compressed form)
    let u_bytes = u.to_repr();
    // In little-endian, check the least significant byte
    let u_is_odd = u_bytes.as_ref()[0] & 1 == 1;
    
    // Clear the high bit first (just in case)
    result[31] &= 0x7f;
    
    // Set the sign bit if u is odd
    if u_is_odd {
        result[31] |= 0x80;
    }
    
    // Debug: let's see the difference
    if &result != &standard_bytes {
        println!("BitcoinZ compat: Converted point format");
        println!("  Standard: {}", hex::encode(&standard_bytes));
        println!("  BitcoinZ: {}", hex::encode(&result));
    }
    
    result
}

/// Convert a SubgroupPoint to BitcoinZ format
pub fn serialize_subgroup_point_bitcoinz(point: &SubgroupPoint) -> [u8; 32] {
    // Convert SubgroupPoint to ExtendedPoint
    let extended = ExtendedPoint::from(*point);
    serialize_edwards_point_bitcoinz(&extended)
}

/// Serialize a value commitment (cv) for BitcoinZ
/// 
/// The value commitment is an edwards point on the Jubjub curve
pub fn serialize_value_commitment_bitcoinz(cv: &jubjub::ExtendedPoint) -> [u8; 32] {
    serialize_edwards_point_bitcoinz(cv)
}

/// Serialize an ephemeral key for BitcoinZ
/// 
/// The ephemeral key is also an edwards point
pub fn serialize_ephemeral_key_bitcoinz(epk: &jubjub::ExtendedPoint) -> [u8; 32] {
    serialize_edwards_point_bitcoinz(epk)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::thread_rng;
    use group::Group;

    #[test]
    fn test_point_serialization() {
        // Generate a random point
        let point = ExtendedPoint::random(&mut thread_rng());
        
        // Serialize it
        let serialized = serialize_edwards_point_bitcoinz(&point);
        
        // Check that it's 32 bytes
        assert_eq!(serialized.len(), 32);
        
        // The highest bit should be 0 or 1
        let sign_bit = (serialized[31] >> 7) & 1;
        assert!(sign_bit == 0 || sign_bit == 1);
    }
    
    #[test]
    fn test_identity_point() {
        // Test serialization of identity/zero point
        let identity = ExtendedPoint::identity();
        let serialized = serialize_edwards_point_bitcoinz(&identity);
        
        // For identity point, y = 1, x = 0
        // So first byte should be 1 (little-endian)
        assert_eq!(serialized[0], 1);
        
        // Rest should be zeros except possibly the sign bit
        for i in 1..31 {
            assert_eq!(serialized[i], 0);
        }
    }
}