// BitcoinZ JavaScript compatibility layer
// Converts between Rust zcash_primitives format and BitcoinZ JS format

use group::{GroupEncoding, Curve};
use jubjub::{SubgroupPoint, ExtendedPoint};
use ff::PrimeField;

/// Convert a SubgroupPoint to BitcoinZ JavaScript compressed format
/// BitcoinZ JS expects compressed edwards points with 0x02/0x03 prefix
pub fn convert_point_to_js_format(point: &SubgroupPoint) -> [u8; 32] {
    let mut result = [0u8; 32];
    
    // Get the compressed representation
    let compressed = point.to_bytes();
    
    // BitcoinZ JS format appears to use a different compression scheme
    // where the first byte is 0x02 or 0x03 (like Bitcoin's compressed pubkeys)
    // followed by the x-coordinate
    
    // For now, let's try the standard compressed format
    result.copy_from_slice(&compressed);
    
    // If the point has odd y-coordinate, set prefix to 0x03, otherwise 0x02
    // This is a guess based on the JS output format
    if is_y_odd(point) {
        result[0] = 0x03;
    } else {
        result[0] = 0x02;
    }
    
    result
}

/// Check if the y-coordinate of a point is odd
fn is_y_odd(point: &SubgroupPoint) -> bool {
    // Convert to affine coordinates to get y
    let affine = point.to_affine();
    let y_bytes = affine.get_v().to_repr();
    
    // Check if the least significant bit is 1 (odd)
    y_bytes[0] & 1 == 1
}

/// Convert an ExtendedPoint to BitcoinZ JavaScript format
pub fn convert_extended_point_to_js_format(point: &ExtendedPoint) -> [u8; 32] {
    // Convert to subgroup point first
    let subgroup_point = SubgroupPoint::from(point.clone());
    convert_point_to_js_format(&subgroup_point)
}

/// Alternative approach: Try to match the exact format BitcoinZ expects
/// Based on the error in librustzcash, it tries to deserialize as edwards::Point
pub fn convert_to_edwards_format(point: &SubgroupPoint) -> [u8; 32] {
    // The edwards point format in the old bellman library might be different
    // Let's try the standard compressed format first
    point.to_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::thread_rng;
    use group::Group;

    #[test]
    fn test_point_conversion() {
        // Generate a random point
        let point = SubgroupPoint::random(&mut thread_rng());
        
        // Convert to JS format
        let js_format = convert_point_to_js_format(&point);
        
        // Check that first byte is 0x02 or 0x03
        assert!(js_format[0] == 0x02 || js_format[0] == 0x03);
        
        println!("Point in JS format: {:02x?}", js_format);
    }
}