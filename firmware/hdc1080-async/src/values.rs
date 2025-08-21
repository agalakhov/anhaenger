//! Readings for the sensor.

/// Private `From` equivalent. This is essentially the same as `From`.
pub(crate) trait PrivateFrom<T: Sized> {
    fn priv_from(x: T) -> Self;
}

/// Temperature reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Temperature(u16);

impl PrivateFrom<[u8; 2]> for Temperature {
    fn priv_from(x: [u8; 2]) -> Self {
        Self(u16::from_be_bytes(x))
    }
}

impl Temperature {
    /// Get temperature in 2^-16 degrees Celsius. For internal use only.
    fn degrees_fp(&self) -> i32 {
        self.0 as i32 * 165 - (1 << 16) * 40
    }

    /// Get temperature in degrees Celsius as floating-point value.
    pub fn degrees_f32(&self) -> f32 {
        self.degrees_fp() as f32 / (1 << 16) as f32
    }

    /// Get approximate temperature in degrees Celsius as integer value.
    pub fn degrees(&self) -> i16 {
        (self.degrees_fp() / (1 << 16)) as i16
    }

    /// Get temperature in 1/10s of degree Celsius.
    pub fn degrees_10(&self) -> i16 {
        (self.degrees_fp() * 10 / (1 << 16)) as i16
    }
}

/// Humidity reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Humidity(u16);

impl PrivateFrom<[u8; 2]> for Humidity {
    fn priv_from(x: [u8; 2]) -> Self {
        Self(u16::from_be_bytes(x))
    }
}

impl Humidity {
    /// Get humidity in percent as floating-point value.
    pub fn percent_f32(&self) -> f32 {
        self.0 as f32 * 100.0 / (1 << 16) as f32
    }

    /// Get approximate humidity in percent as integer value.
    pub fn percent(&self) -> u8 {
        (self.0 as u32 * 100 / (1 << 16)) as u8
    }

    /// Get humidity in 1/10s of percent.
    pub fn percent_10(&self) -> u16 {
        (self.0 as u32 * 1000 / (1 << 16)) as u16
    }
}

impl PrivateFrom<[u8; 4]> for (Temperature, Humidity) {
    fn priv_from(x: [u8; 4]) -> Self {
        let [t1, t2, h1, h2] = x;
        let t = Temperature::priv_from([t1, t2]);
        let h = Humidity::priv_from([h1, h2]);
        (t, h)
    }
}
