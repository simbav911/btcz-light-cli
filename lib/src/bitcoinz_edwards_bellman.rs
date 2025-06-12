/// BitcoinZ Edwards Point Serialization (Bellman 0.1.0 format)
/// 
/// This module implements the exact edwards point serialization format
/// used by BitcoinZ's bellman 0.1.0 library.

use jubjub::{ExtendedPoint, AffinePoint};
use ff::PrimeField;
use group::Curve;
use std::io;
use std::convert::TryInto;
use hex;

/// Check if a field element representation is odd
/// This matches BitcoinZ's is_odd() implementation
fn is_repr_odd(repr: &[u8; 32]) -> bool {
    // In little-endian, the least significant bit is in the first byte
    repr[0] & 1 == 1
}

/// Serialize an edwards point in BitcoinZ's bellman 0.1.0 format
/// 
/// The format is:
/// - Convert point to affine coordinates (x, y)
/// - Check if x coordinate representation is odd
/// - Write y coordinate in little-endian
/// - If x is odd, set bit 63 of the 4th u64 (when viewed as array of u64s)
pub fn write_edwards_point_bellman<W: io::Write>(
    point: &ExtendedPoint,
    mut writer: W,
) -> io::Result<()> {
    // Convert to affine coordinates
    let affine = AffinePoint::from(point);
    
    // Get x and y coordinates
    let x = affine.get_u();
    let y = affine.get_v();
    
    // Get the representations
    let x_repr = x.to_repr();
    let mut y_repr = y.to_repr();
    
    // Check if x is odd
    let x_bytes = x_repr.as_ref();
    let x_is_odd = is_repr_odd(&x_bytes.try_into().expect("x_repr should be 32 bytes"));
    
    println!("BitcoinZ bellman point write:");
    println!("  x bytes: {}", hex::encode(x_bytes));
    println!("  y bytes (before): {}", hex::encode(y_repr.as_ref()));
    println!("  x is odd: {}", x_is_odd);
    
    // BitcoinZ bellman 0.1.0 implementation:
    // if x_repr.is_odd() {
    //     y_repr.as_mut()[3] |= 0x8000000000000000u64;
    // }
    // This treats y_repr as an array of u64s!
    
    if x_is_odd {
        // We need to simulate treating the bytes as a u64 array
        // In BitcoinZ's PrimeFieldRepr, as_mut() returns &mut [u64; 4]
        // We need to set bit 63 of the 4th u64 (index 3)
        
        let y_bytes = y_repr.as_mut();
        
        // The 4th u64 in little-endian starts at byte 24
        // Read it, set bit 63, write it back
        let before = u64::from_le_bytes([
            y_bytes[24], y_bytes[25], y_bytes[26], y_bytes[27],
            y_bytes[28], y_bytes[29], y_bytes[30], y_bytes[31]
        ]);
        
        let after = before | 0x8000000000000000u64;
        
        let after_bytes = after.to_le_bytes();
        y_bytes[24..32].copy_from_slice(&after_bytes);
        
        println!("  Setting sign bit in bellman format");
        println!("  4th u64 before: 0x{:016x}", before);
        println!("  4th u64 after:  0x{:016x}", after);
        println!("  New y bytes: {}", hex::encode(y_repr.as_ref()));
    }
    
    // Write y representation in little-endian
    writer.write_all(y_repr.as_ref())
}


#[cfg(test)]
mod tests {
    use super::*;
    use rand::thread_rng;
    use group::Group;
    
    #[test]
    fn test_edwards_point_write() {
        let mut rng = thread_rng();
        
        // Test with random points
        for _ in 0..10 {
            let point = ExtendedPoint::random(&mut rng);
            
            let mut bytes = Vec::new();
            write_edwards_point_bellman(&point, &mut bytes).unwrap();
            
            assert_eq!(bytes.len(), 32);
        }
        
        // Test with identity
        let identity = ExtendedPoint::identity();
        let mut bytes = Vec::new();
        write_edwards_point_bellman(&identity, &mut bytes).unwrap();
        
        assert_eq!(bytes.len(), 32);
    }
}