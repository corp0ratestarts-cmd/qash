//! Fixed-point arithmetic (_p), deterministic and checked.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverflowError;

pub const SCALE: i128 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FixedPoint {
    value: i128,
}

impl FixedPoint {
    pub const ZERO: FixedPoint = FixedPoint { value: 0 };
    pub const ONE: FixedPoint = FixedPoint { value: SCALE };

    #[inline]
    pub const fn from_raw(raw: i128) -> Self {
        FixedPoint { value: raw }
    }

    #[inline]
    pub const fn raw(self) -> i128 {
        self.value
    }

    #[inline]
    pub const fn is_non_negative(self) -> bool {
        self.value >= 0
    }

    #[inline]
    pub fn to_i64(self) -> Result<i64, OverflowError> {
        if self.value > i64::MAX as i128 || self.value < i64::MIN as i128 {
            Err(OverflowError)
        } else {
            Ok(self.value as i64)
        }
    }

    #[inline]
    pub fn checked_add(self, other: Self) -> Result<Self, OverflowError> {
        let sum = self.value.checked_add(other.value).ok_or(OverflowError)?;
        Ok(FixedPoint { value: sum })
    }

    #[inline]
    pub fn checked_sub(self, other: Self) -> Result<Self, OverflowError> {
        let diff = self.value.checked_sub(other.value).ok_or(OverflowError)?;
        Ok(FixedPoint { value: diff })
    }

    /// Computes floor((a*b)/SCALE).
    #[inline]
    pub fn checked_mul(self, other: Self) -> Result<Self, OverflowError> {
        let product = self.value.checked_mul(other.value).ok_or(OverflowError)?;
        let rescaled = floor_div_i128(product, SCALE)?;
        Ok(FixedPoint { value: rescaled })
    }

    /// Computes floor((a*SCALE)/b).
    #[inline]
    pub fn checked_div(self, other: Self) -> Result<Self, OverflowError> {
        if other.value == 0 {
            return Err(OverflowError);
        }
        let scaled = self.value.checked_mul(SCALE).ok_or(OverflowError)?;
        let q = floor_div_i128(scaled, other.value)?;
        Ok(FixedPoint { value: q })
    }

    #[inline]
    pub const fn min(self, other: Self) -> Self {
        if self.value <= other.value {
            self
        } else {
            other
        }
    }

    #[inline]
    pub const fn max(self, other: Self) -> Self {
        if self.value >= other.value {
            self
        } else {
            other
        }
    }
}

/// Floor division toward -∞.
pub fn floor_div_i128(a: i128, b: i128) -> Result<i128, OverflowError> {
    if b == 0 {
        return Err(OverflowError);
    }
    // i128::MIN / -1 overflows
    if a == i128::MIN && b == -1 {
        return Err(OverflowError);
    }
    let q = a / b;
    let r = a % b;
    if r != 0 && ((a ^ b) < 0) {
        Ok(q - 1)
    } else {
        Ok(q)
    }
}

pub const FIXED_POINT_WIRE_BYTES: u32 = 16;

#[inline]
pub fn encode_fixed_point(fp: FixedPoint) -> [u8; 16] {
    fp.raw().to_le_bytes()
}

#[inline]
pub fn decode_fixed_point(bytes: [u8; 16]) -> FixedPoint {
    FixedPoint::from_raw(i128::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floor_div_examples() {
        assert_eq!(floor_div_i128(7, 2), Ok(3));
        assert_eq!(floor_div_i128(-7, 2), Ok(-4));
        assert_eq!(floor_div_i128(7, -2), Ok(-4));
        assert_eq!(floor_div_i128(-7, -2), Ok(3));
    }

    #[test]
    fn mul_rescales() {
        let half = FixedPoint::from_raw(500_000);
        assert_eq!(half.checked_mul(half).map(|x| x.raw()), Ok(250_000));
    }

    #[test]
    fn checked_add_overflow_is_error() {
        let a = FixedPoint::from_raw(i128::MAX);
        let b = FixedPoint::from_raw(1);
        assert_eq!(a.checked_add(b), Err(OverflowError));
    }

    #[test]
    fn checked_sub_overflow_is_error() {
        let a = FixedPoint::from_raw(i128::MIN);
        let b = FixedPoint::from_raw(1);
        assert_eq!(a.checked_sub(b), Err(OverflowError));
    }

    #[test]
    fn checked_mul_overflow_is_error() {
        let a = FixedPoint::from_raw(i128::MAX);
        let b = FixedPoint::from_raw(2);
        assert_eq!(a.checked_mul(b), Err(OverflowError));
    }

    #[test]
    fn checked_div_overflow_is_error() {
        let a = FixedPoint::from_raw(i128::MAX);
        let b = FixedPoint::from_raw(1);
        assert_eq!(a.checked_div(b), Err(OverflowError));
    }

    #[test]
    fn checked_div_by_zero_is_error() {
        let a = FixedPoint::from_raw(123_456);
        assert_eq!(a.checked_div(FixedPoint::ZERO), Err(OverflowError));
    }

    #[test]
    fn floor_div_matches_euclidean_semantics_for_sign_mixed_inputs() {
        let cases = [
            (-1_i128, 2_i128, -1_i128),
            (1, -2, -1),
            (-3, 2, -2),
            (3, -2, -2),
            (-5, 3, -2),
            (5, -3, -2),
        ];

        for (a, b, expected_floor_q) in cases {
            assert_eq!(floor_div_i128(a, b), Ok(expected_floor_q));
        }
    }

    #[test]
    fn floor_div_i128_min_div_negative_one_is_error() {
        assert_eq!(floor_div_i128(i128::MIN, -1), Err(OverflowError));
    }

    #[test]
    fn to_i64_bounds_are_checked() {
        assert_eq!(
            FixedPoint::from_raw(i64::MAX as i128).to_i64(),
            Ok(i64::MAX)
        );
        assert_eq!(
            FixedPoint::from_raw(i64::MIN as i128).to_i64(),
            Ok(i64::MIN)
        );
        assert_eq!(
            FixedPoint::from_raw(i64::MAX as i128 + 1).to_i64(),
            Err(OverflowError)
        );
        assert_eq!(
            FixedPoint::from_raw(i64::MIN as i128 - 1).to_i64(),
            Err(OverflowError)
        );
    }
}
