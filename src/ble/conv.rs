use num_traits::float::FloatCore;

pub trait BleScalarReprExt {
    fn ble_serialize(&self, multiplier: i32, decimal_exponent: i32, binary_exponent: i32) -> Self;
    fn ble_deserialize(&self, multiplier: i32, decimal_exponent: i32, binary_exponent: i32) -> Self;
}

impl BleScalarReprExt for f32 {
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

pub trait ConvExt {
    fn as_voltage(&self) -> u16;
    fn as_temp(&self) -> i16;
}

impl ConvExt for f32 {
    fn as_voltage(&self) -> u16 {
        // Represented values: M = 1, d = 0, b = -6
        self.ble_serialize(1, 0, -6) as u16
    }
    fn as_temp(&self) -> i16 {
        //  M = 1, d = -2, b = 0
        self.ble_serialize(1, -2, 0) as i16
    }
}