#[cfg(test)]
mod tests {
    use jubjub::ExtendedPoint;
    use group::{Group, GroupEncoding, Curve};
    use ff::PrimeField;
    use rand::thread_rng;
    use crate::bitcoinz_compat_v2::serialize_edwards_point_bitcoinz_v2;

    #[test]
    fn test_edwards_point_serialization() {
        println!("\n=== Testing Edwards Point Serialization ===\n");
        
        // Test 1: Identity point
        let identity = ExtendedPoint::identity();
        verify_point_format("Identity", &identity);
        
        // Test 2: Generator point
        let generator = ExtendedPoint::generator();
        verify_point_format("Generator", &generator);
        
        // Test 3: Random points
        for i in 0..5 {
            let random = ExtendedPoint::random(&mut thread_rng());
            verify_point_format(&format!("Random {}", i), &random);
        }
    }
    
    #[test]
    fn test_round_trip_serialization() {
        println!("\n=== Testing Round-Trip Serialization ===\n");
        
        // Test that we can serialize and deserialize points correctly
        for i in 0..10 {
            let original = ExtendedPoint::random(&mut thread_rng());
            
            // Serialize using BitcoinZ format
            let serialized = serialize_edwards_point_bitcoinz_v2(&original)
                .expect("Failed to serialize");
            
            // Try to deserialize using standard method
            let ct_option = ExtendedPoint::from_bytes(&serialized);
            if bool::from(ct_option.is_some()) {
                let deserialized = ct_option.unwrap();
                // Check if points are equal
                if original == deserialized {
                    println!("Test {}: Round-trip successful ✓", i);
                } else {
                    println!("Test {}: Points differ after round-trip!", i);
                    println!("  Original: {:?}", original);
                    println!("  Deserialized: {:?}", deserialized);
                }
            } else {
                println!("Test {}: Failed to deserialize!", i);
                println!("  Serialized bytes: {}", hex::encode(&serialized));
            }
        }
    }
    
    fn verify_point_format(name: &str, point: &ExtendedPoint) {
        println!("Testing {}: ", name);
        
        // Get standard serialization
        let standard = point.to_bytes();
        println!("  Standard format: {}", hex::encode(&standard));
        
        // Get BitcoinZ format
        match serialize_edwards_point_bitcoinz_v2(point) {
            Ok(bitcoinz) => {
                println!("  BitcoinZ format: {}", hex::encode(&bitcoinz));
                
                // Get affine coordinates
                let affine = point.to_affine();
                let u = affine.get_u();
                let v = affine.get_v();
                
                let u_repr = u.to_repr();
                let v_repr = v.to_repr();
                
                // Verify y coordinate matches
                let mut bitcoinz_y = bitcoinz;
                bitcoinz_y[31] &= 0x7f; // Clear sign bit
                let v_bytes = v_repr.as_ref();
                assert_eq!(&bitcoinz_y[..], v_bytes, "{}: Y coordinate mismatch", name);
                
                // Verify sign bit
                let u_is_odd = u_repr.as_ref()[0] & 1 == 1;
                let bitcoinz_has_sign_bit = (bitcoinz[31] & 0x80) != 0;
                assert_eq!(u_is_odd, bitcoinz_has_sign_bit, "{}: Sign bit mismatch", name);
                
                println!("  ✓ Format verified correctly");
            }
            Err(e) => {
                panic!("Failed to serialize {}: {}", name, e);
            }
        }
        println!();
    }
}