use num_traits::float::FloatCore;

pub trait BleScalarRepr {
    fn ble_serialize(&self, multiplier: i32, decimal_exponent: i32, binary_exponent: i32) -> Self;
    fn ble_deserialize(&self, multiplier: i32, decimal_exponent: i32, binary_exponent: i32) -> Self;
}

impl BleScalarRepr for f32 {
    /// Inverse for
    /// R = C * M * 10^d * 2^b
    fn ble_serialize(&self, multiplier: i32, decimal_exponent: i32, binary_exponent: i32) -> Self {
        let mut result = *self / (multiplier as f32);
        result /= 10f32.powi(decimal_exponent);
        result /= 2f32.powi(binary_exponent);

        result
    }

    /// R = C * M * 10^d * 2^b
    fn ble_deserialize(&self, multiplier: i32, decimal_exponent: i32, binary_exponent: i32) -> Self {
        let mut result = *self * (multiplier as f32);

        result *= 10f32.powi(decimal_exponent);
        result *= 2f32.powi(binary_exponent);

        result
    }
}
