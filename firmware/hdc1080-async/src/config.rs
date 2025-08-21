//! HDC1080 configuration handling.

/// Chip configuration.
#[derive(Debug, Clone)]
pub struct Config {
    pub t_resolution: TemperatureResolution,
    pub h_resolution: HumidityResolution,
    pub drying_heater: DryingHeater,
    pub acquisition: Acquisition,
}

impl Config {
    pub(crate) fn as_bits(&self) -> u16 {
        self.t_resolution.as_config_bits()
            | self.h_resolution.as_config_bits()
            | self.drying_heater.as_config_bits()
            | self.acquisition.as_config_bits()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            t_resolution: TemperatureResolution::Bits11,
            h_resolution: HumidityResolution::Bits14,
            drying_heater: DryingHeater::Off,
            acquisition: Acquisition::Simultaneous,
        }
    }
}

/// Resolution of temperature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemperatureResolution {
    /// 11 bits
    Bits11,
    /// 14 bits
    Bits14,
}

impl TemperatureResolution {
    fn as_config_bits(&self) -> u16 {
        (match self {
            TemperatureResolution::Bits11 => 1,
            TemperatureResolution::Bits14 => 0,
        }) << 10 // config register bit 10
    }

    pub(crate) fn t_acq_us(&self) -> u32 {
        match self {
            TemperatureResolution::Bits11 => 3650,
            TemperatureResolution::Bits14 => 6350,
        }
    }
}

/// Resolution of humidity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HumidityResolution {
    /// 8 bits
    Bits8,
    /// 11 bits
    Bits11,
    /// 14 bits
    Bits14,
}

impl HumidityResolution {
    fn as_config_bits(&self) -> u16 {
        (match self {
            HumidityResolution::Bits8 => 2,
            HumidityResolution::Bits11 => 1,
            HumidityResolution::Bits14 => 0,
        }) << 8 // config register bits 8, 9
    }

    pub(crate) fn t_acq_us(&self) -> u32 {
        match self {
            HumidityResolution::Bits8 => 2500,
            HumidityResolution::Bits11 => 3850,
            HumidityResolution::Bits14 => 6450,
        }
    }
}

/// Drying heater on/off switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DryingHeater {
    /// Drying heater is inactive.
    Off,
    /// Drying heater is active.
    On,
}

impl DryingHeater {
    fn as_config_bits(&self) -> u16 {
        (match self {
            DryingHeater::On => 1,
            DryingHeater::Off => 0,
        }) << 13 // config register bit 13
    }
}

impl From<bool> for DryingHeater {
    fn from(b: bool) -> Self {
        if b {
            DryingHeater::On
        } else {
            DryingHeater::Off
        }
    }
}

impl From<DryingHeater> for bool {
    fn from(h: DryingHeater) -> Self {
        h == DryingHeater::On
    }
}

/// Acquisition configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acquisition {
    /// Get temperature and humidity at the same time in one I2C request.
    Simultaneous,
    /// Get temperature and humidity separately in two requests.
    Separate,
}

impl Acquisition {
    fn as_config_bits(&self) -> u16 {
        (match self {
            Acquisition::Simultaneous => 0,
            Acquisition::Separate => 1,
        }) << 12 // config register bit 12
    }
}
