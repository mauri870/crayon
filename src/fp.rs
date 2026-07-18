// Cray-1 64-bit floating point format (signed-magnitude, biased exponent):
//
//   Bit 63    : sign (0 = positive, 1 = negative)
//   Bits 62-48: biased exponent (15 bits, bias = 0x4000 = 16384)
//   Bits 47-0 : coefficient (48 bits, binary point between bit 47 and 46)
//
// Normalized form: bit 47 of the coefficient is 1.
// Zero: all bits 0 (exp=0, coeff=0).
//
// A normalized non-zero value represents: (-1)^sign * (coeff/2^48) * 2^(exp - 16384)
// Since bit 47 = 1, coeff/2^48 is in [0.5, 1.0).

/// Normalize a Cray-1 FP word: shift coefficient left until bit 47 is set.
pub fn normalize(fp: u64) -> u64 {
    let sign = fp & (1 << 63);
    let mut exp = ((fp >> 48) & 0x7FFF) as i64;
    let mut coeff = fp & 0x0000_FFFF_FFFF_FFFF;

    if coeff == 0 {
        return 0;
    }
    while (coeff >> 47) == 0 {
        coeff <<= 1;
        exp -= 1;
        if exp <= 0 {
            return 0; // underflow -> zero
        }
    }
    sign | ((exp as u64) << 48) | coeff
}

/// Flip the sign bit.
pub fn negate(fp: u64) -> u64 {
    fp ^ (1 << 63)
}

/// Convert a Cray-1 FP word to f64. Normalizes before converting.
pub fn to_f64(fp: u64) -> f64 {
    let fp = normalize(fp);
    let sign = fp >> 63;
    let cray_exp = (fp >> 48) & 0x7FFF;
    let coeff = fp & 0x0000_FFFF_FFFF_FFFF;

    if cray_exp == 0 || coeff == 0 {
        return if sign != 0 { -0.0 } else { 0.0 };
    }

    // Derive f64 biased exponent: cray_exp = f64_exp + 15362
    let f64_exp = cray_exp as i64 - 15362;
    if f64_exp <= 0 || f64_exp >= 0x7FF {
        return if sign != 0 { -0.0 } else { 0.0 };
    }

    // f64 fraction = lower 47 bits of coeff shifted left 5 (52 - 47)
    let f64_frac = (coeff & ((1u64 << 47) - 1)) << 5;
    let f64_bits = (sign << 63) | ((f64_exp as u64) << 52) | f64_frac;
    f64::from_bits(f64_bits)
}

/// Convert an f64 to the nearest Cray-1 FP word, truncating to 48-bit coefficient.
pub fn from_f64(f: f64) -> u64 {
    if f == 0.0 || f.is_nan() {
        return 0;
    }
    if f.is_infinite() {
        let sign: u64 = if f < 0.0 { 1 } else { 0 };
        return (sign << 63) | (0x7FFF_u64 << 48) | 0x0000_FFFF_FFFF_FFFF;
    }

    let bits = f.to_bits();
    let sign = bits >> 63;
    let f64_exp = (bits >> 52) & 0x7FF;
    let f64_frac = bits & 0x000F_FFFF_FFFF_FFFF;

    if f64_exp == 0 {
        return 0; // denormal -> zero
    }

    // cray_exp = f64_exp + 15362
    let cray_exp = f64_exp as i64 + 15362;
    if cray_exp <= 0 {
        return 0; // underflow
    }
    if cray_exp > 0x7FFF {
        return (sign << 63) | (0x7FFF_u64 << 48) | 0x0000_FFFF_FFFF_FFFF; // overflow
    }

    // coefficient: bit 47 set (normalized), bits 46-0 from top 47 bits of f64 fraction
    let coeff = (1u64 << 47) | (f64_frac >> 5);

    (sign << 63) | ((cray_exp as u64) << 48) | coeff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_one() {
        assert_eq!(to_f64(from_f64(1.0)), 1.0);
    }

    #[test]
    fn roundtrip_neg_one() {
        assert_eq!(to_f64(from_f64(-1.0)), -1.0);
    }

    #[test]
    fn roundtrip_two_point_five() {
        assert_eq!(to_f64(from_f64(2.5)), 2.5);
    }

    #[test]
    fn roundtrip_half() {
        assert_eq!(to_f64(from_f64(0.5)), 0.5);
    }

    #[test]
    fn zero_is_zero() {
        assert_eq!(from_f64(0.0), 0);
        assert_eq!(to_f64(0), 0.0);
    }

    #[test]
    fn negate_flips_sign() {
        let one = from_f64(1.0);
        assert_eq!(to_f64(negate(one)), -1.0);
    }

    #[test]
    fn normalize_shifts_up() {
        // Construct an unnormalized FP word for 1.0: put the coefficient in the
        // low bit instead of bit 47, and raise the exponent accordingly.
        let one = from_f64(1.0);
        let exp = (one >> 48) & 0x7FFF;
        let coeff = one & 0x0000_FFFF_FFFF_FFFF;
        // Shift coefficient right by 4 and compensate with exponent+4
        let unnorm = ((exp + 4) << 48) | (coeff >> 4);
        assert_eq!(to_f64(normalize(unnorm)), 1.0);
    }

    #[test]
    fn add_via_f64() {
        let a = from_f64(1.5);
        let b = from_f64(2.5);
        let result = from_f64(to_f64(a) + to_f64(b));
        assert_eq!(to_f64(result), 4.0);
    }
}
