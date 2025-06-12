// Test program to verify edwards point serialization compatibility

use jubjub::{ExtendedPoint, SubgroupPoint};
use group::Group;
use ff::PrimeField;
use rand::thread_rng;

// Include our compatibility module
#[path = "lib/src/bitcoinz_compat_v2.rs"]
mod bitcoinz_compat_v2;
use bitcoinz_compat_v2::serialize_edwards_point_bitcoinz_v2;

fn main() {
    println!("=== Testing Edwards Point Serialization ===\n");
    
    // Test 1: Identity point
    let identity = ExtendedPoint::identity();
    test_point("Identity", &identity);
    
    // Test 2: Generator point
    let generator = ExtendedPoint::generator();
    test_point("Generator", &generator);
    
    // Test 3: Random point
    let random = ExtendedPoint::random(&mut thread_rng());
    test_point("Random", &random);
    
    // Test 4: Specific test point to match BitcoinZ format
    // Create a point with known coordinates to verify serialization
    let test_point_1 = generator * jubjub::Fr::from(12345u64);
    test_point("Test Point 1", &test_point_1);
}

fn test_point(name: &str, point: &ExtendedPoint) {
    println!("Testing {}: ", name);
    
    // Get standard serialization
    let standard = point.to_bytes();
    println!("  Standard format: {}", hex::encode(&standard));
    
    // Get BitcoinZ format
    match serialize_edwards_point_bitcoinz_v2(point) {
        Ok(bitcoinz) => {
            println!("  BitcoinZ format: {}", hex::encode(&bitcoinz));
            
            // Compare the formats
            if standard != bitcoinz {
                println!("  Formats differ! Analyzing...");
                
                // Check the sign bit
                let standard_sign = (standard[31] & 0x80) != 0;
                let bitcoinz_sign = (bitcoinz[31] & 0x80) != 0;
                println!("    Standard sign bit: {}", standard_sign);
                println!("    BitcoinZ sign bit: {}", bitcoinz_sign);
                
                // Check the y coordinate (without sign bit)
                let mut standard_y = standard;
                standard_y[31] &= 0x7f;
                let mut bitcoinz_y = bitcoinz;
                bitcoinz_y[31] &= 0x7f;
                
                if standard_y == bitcoinz_y {
                    println!("    Y coordinates match (sign bit differs)");
                } else {
                    println!("    Y coordinates differ!");
                }
            } else {
                println!("  Formats match!");
            }
        }
        Err(e) => {
            println!("  Error: {}", e);
        }
    }
    
    // Get affine coordinates for analysis
    let affine = point.to_affine();
    let u = affine.get_u();
    let v = affine.get_v();
    
    let u_repr = u.to_repr();
    let v_repr = v.to_repr();
    
    println!("  Affine coordinates:");
    println!("    u (x): {}", hex::encode(u_repr.as_ref()));
    println!("    v (y): {}", hex::encode(v_repr.as_ref()));
    println!("    u is odd: {}", u_repr.as_ref()[0] & 1 == 1);
    println!();
}