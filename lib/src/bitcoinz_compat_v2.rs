/// BitcoinZ Compatibility Layer V2
/// 
/// This module provides exact format conversion between modern zcash libraries
/// and BitcoinZ's old bellman 0.1.0 format.

use jubjub::{AffinePoint, ExtendedPoint, SubgroupPoint, Fr as JubjubFr};
use ff::{Field, PrimeField};
use group::{Curve, GroupEncoding};
use hex;

/// The key insight: BitcoinZ's old format uses different coordinate systems
/// and serialization rules than modern jubjub.
///
/// Modern jubjub (twisted Edwards): 
///   - Uses (u, v) coordinates where u = x/z, v = y/z
///   - Serializes as compressed y-coordinate with x-sign
///
/// BitcoinZ's old format (also twisted Edwards but different convention):
///   - Uses direct (x, y) coordinates in affine form
///   - Stores y-coordinate with x's parity bit in position 255
///   - Uses different is_odd check (LSB of first limb)

/// Convert modern jubjub point to BitcoinZ's edwards format
/// 
/// BitcoinZ's edwards point format (from their source):
/// 1. Convert point to affine coordinates (x, y)
/// 2. Store y-coordinate in little-endian format
/// 3. If x is odd (LSB of first u64 is 1), set bit 255 (MSB of last byte)
pub fn serialize_edwards_point_bitcoinz_v2(point: &ExtendedPoint) -> Result<[u8; 32], String> {
    // Convert to affine to get actual coordinates
    let affine = point.to_affine();
    
    // In modern jubjub, affine points have (u, v) coordinates
    // In BitcoinZ's old format, they have (x, y) coordinates
    // These are the same coordinates, just different naming:
    // BitcoinZ's x = modern u
    // BitcoinZ's y = modern v
    
    let x = affine.get_u(); // BitcoinZ calls this 'x'
    let y = affine.get_v(); // BitcoinZ calls this 'y'
    
    // Serialize y coordinate in little-endian
    let y_repr = y.to_repr();
    let mut result = [0u8; 32];
    result.copy_from_slice(&y_repr.as_ref());
    
    // Clear the high bit to ensure it's clean
    result[31] &= 0x7f;
    
    // Check if x is odd using BitcoinZ's method:
    // is_odd() checks if self.0[0] & 1 == 1
    let x_repr = x.to_repr();
    let x_is_odd = x_repr.as_ref()[0] & 1 == 1;
    
    // Set the sign bit (bit 255) if x is odd
    if x_is_odd {
        result[31] |= 0x80;
    }
    
    Ok(result)
}

/// Alternative approach: Try to match the exact bit pattern
pub fn serialize_edwards_point_bitcoinz_v3(point: &ExtendedPoint) -> Result<[u8; 32], String> {
    // Get the standard compressed representation
    let compressed = point.to_bytes();
    
    // The difference might be in how the sign bit is encoded
    // Modern: sign in bit 255 (MSB of last byte)
    // BitcoinZ: also in bit 255, but different parity check
    
    let mut result = compressed;
    
    // Extract the sign bit from modern format
    let has_sign_bit = (compressed[31] & 0x80) != 0;
    
    // Clear and re-set based on different parity logic
    result[31] &= 0x7f;
    
    // In BitcoinZ's old format, they check is_odd differently
    // Let's try inverting the sign bit
    if !has_sign_bit {  // If modern format says even, BitcoinZ might expect odd
        result[31] |= 0x80;
    }
    
    Ok(result)
}

/// Exact BitcoinZ format: set bit on u64 array before little-endian serialization
pub fn serialize_edwards_point_bitcoinz_exact(point: &ExtendedPoint) -> Result<[u8; 32], String> {
    // Get affine coordinates
    let affine = point.to_affine();
    let x = affine.get_u();
    let y = affine.get_v();
    
    // Get the field element representations
    let x_repr = x.to_repr();
    let y_repr = y.to_repr();
    
    // BitcoinZ stores the y coordinate with x's sign bit
    // They set the bit on the u64 array BEFORE serialization
    let mut result = [0u8; 32];
    
    // Get the underlying u64 limbs
    // For modern ff/jubjub, the repr is stored as bytes, not u64 array
    // We need to convert back to u64 array
    let y_bytes = y_repr.as_ref();
    let x_bytes = x_repr.as_ref();
    
    // Check if x is odd using the first byte's LSB
    let x_is_odd = x_bytes[0] & 1 == 1;
    
    // Convert y bytes to u64 array (little-endian)
    let mut y_limbs = [0u64; 4];
    for i in 0..4 {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&y_bytes[i * 8..(i + 1) * 8]);
        y_limbs[i] = u64::from_le_bytes(bytes);
    }
    
    if x_is_odd {
        // Set bit 255 by setting the MSB of the last u64
        y_limbs[3] |= 0x8000000000000000u64;
    }
    
    // Now write the modified u64 array as little-endian bytes
    for i in 0..4 {
        let bytes = y_limbs[i].to_le_bytes();
        result[i * 8..(i + 1) * 8].copy_from_slice(&bytes);
    }
    
    Ok(result)
}

/// Try direct coordinate extraction and custom serialization
pub fn serialize_edwards_point_bitcoinz_v4(point: &ExtendedPoint) -> Result<[u8; 32], String> {
    // Convert to affine
    let affine = point.to_affine();
    
    // In Edwards curves, we typically store y and sign of x
    // But the exact representation matters
    
    // Get coordinates
    let u = affine.get_u();
    let v = affine.get_v();
    
    // BitcoinZ might expect a different normalization
    // Let's try storing u as-is with v's sign (opposite of usual)
    let mut result = [0u8; 32];
    
    // Write u coordinate (instead of v)
    let u_bytes = u.to_repr();
    result.copy_from_slice(&u_bytes.as_ref());
    
    // Set sign based on v
    result[31] &= 0x7f;
    let v_bytes = v.to_repr();
    if v_bytes.as_ref()[0] & 1 == 1 {
        result[31] |= 0x80;
    }
    
    Ok(result)
}

/// Debug function to show the differences
pub fn debug_point_formats(point: &ExtendedPoint) {
    println!("=== Point Format Analysis ===");
    
    // Standard format
    let standard = point.to_bytes();
    println!("Standard format: {}", hex::encode(&standard));
    
    // Try different conversions
    if let Ok(v2) = serialize_edwards_point_bitcoinz_v2(point) {
        println!("BitcoinZ v2:     {}", hex::encode(&v2));
        if v2 != standard {
            println!("  Difference detected!");
        }
    }
    
    if let Ok(v3) = serialize_edwards_point_bitcoinz_v3(point) {
        println!("BitcoinZ v3:     {}", hex::encode(&v3));
        if v3 != standard {
            println!("  Difference detected!");
        }
    }
    
    if let Ok(v4) = serialize_edwards_point_bitcoinz_v4(point) {
        println!("BitcoinZ v4:     {}", hex::encode(&v4));
        if v4 != standard {
            println!("  Difference detected!");
        }
    }
    
    if let Ok(exact) = serialize_edwards_point_bitcoinz_exact(point) {
        println!("BitcoinZ exact:  {}", hex::encode(&exact));
        if exact != standard {
            println!("  Difference detected!");
        }
    }
    
    // Show coordinate details
    let affine = point.to_affine();
    let u = affine.get_u();
    let v = affine.get_v();
    
    let u_bytes = u.to_repr();
    let v_bytes = v.to_repr();
    
    println!("\nCoordinate analysis:");
    println!("  u first byte: 0x{:02x} (odd: {})", u_bytes.as_ref()[0], u_bytes.as_ref()[0] & 1);
    println!("  v first byte: 0x{:02x} (odd: {})", v_bytes.as_ref()[0], v_bytes.as_ref()[0] & 1);
    println!("  Standard sign bit: {}", (standard[31] & 0x80) != 0);
}

/// Test with known values
#[cfg(test)]
mod tests {
    use super::*;
    use rand::thread_rng;
    use group::Group;

    #[test]
    fn test_format_differences() {
        // Test with identity point
        let identity = ExtendedPoint::identity();
        debug_point_formats(&identity);
        
        // Test with generator
        let generator = ExtendedPoint::generator();
        debug_point_formats(&generator);
        
        // Test with random point
        let random = ExtendedPoint::random(&mut thread_rng());
        debug_point_formats(&random);
    }
}