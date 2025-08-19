#![deny(unsafe_code)]
#![no_std]

use defmt::{Debug2Format, info};
use core::fmt::Debug;

use embedded_hal::{delay::DelayNs as DelayBlocking, i2c::I2c as I2cBlocking};
use embedded_hal_async::{delay::DelayNs as DelayAsync, i2c::I2c as I2cAsync};

mod config;
mod values;
pub use config::*;
pub use values::*;

const I2C_ADDRESS: u8 = 0x40;
const EXTRA_DELAY: u32 = 8000; // microseconds
const RESET_DELAY: u32 = 10000; // microseconds

/// Chip identification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Identification {
    /// Chip manufacturer. Always 0x5449 (Texas Instruments).
    pub manufacturer: u16,
    /// Chip product code. Always 0x1050 for HDC1080.
    pub product: u16,
    /// Chip serial number in big-endian byte order.
    pub serial: [u8; 5],
}

impl Identification {
    /// Check if the chip is a working HDC1080.
    pub fn is_valid(&self) -> bool {
        self.manufacturer == 0x5449 && self.product == 0x1050
    }

    /// Register array to construct from.
    const REGISTERS: [Register; 5] = [
        Register::Manufacturer,
        Register::DeviceId,
        Register::SerialId1,
        Register::SerialId2,
        Register::SerialId3,
    ];

    /// Convert register words into binary data.
    fn from_registers(data: [u16; 5]) -> Self {
        let mut serial = [0_u8; 5];
        serial[0..2].copy_from_slice(&data[2].to_be_bytes());
        serial[2..4].copy_from_slice(&data[3].to_be_bytes());
        serial[4] = data[4] as u8;
        Self {
            manufacturer: data[0],
            product: data[1],
            serial,
        }
    }
}

/// HDC1080 chip registers.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Register {
    Temperature = 0x00,
    Humidity = 0x01,
    Configuration = 0x02,
    SerialId1 = 0xFB,
    SerialId2 = 0xFC,
    SerialId3 = 0xFD,
    Manufacturer = 0xFE,
    DeviceId = 0xFF,
}

const RESET_COMMAND: u16 = 1 << 15;
const BATT_LOW_BIT: u16 = 1 << 11;

trait Request: Sized + PrivateFrom<Self::Buf> {
    type Buf: Default + AsMut<[u8]> + Debug;
    const REG: Register;
    fn get_delay_us(config: &Config) -> u32;
}

impl Request for Temperature {
    type Buf = [u8; 2];
    const REG: Register = Register::Temperature;
    fn get_delay_us(config: &Config) -> u32 {
        config.t_resolution.t_acq_us() + EXTRA_DELAY
    }
}

impl Request for Humidity {
    type Buf = [u8; 2];
    const REG: Register = Register::Humidity;
    fn get_delay_us(config: &Config) -> u32 {
        config.h_resolution.t_acq_us() + EXTRA_DELAY
    }
}

impl Request for (Temperature, Humidity) {
    type Buf = [u8; 4];
    const REG: Register = Register::Temperature;
    fn get_delay_us(config: &Config) -> u32 {
        config
            .t_resolution
            .t_acq_us()
            .max(config.h_resolution.t_acq_us())
            + EXTRA_DELAY
    }
}

/// The HDC1080 temperature and humidity sensor driver.
pub struct Hdc1080<I2C, D> {
    i2c: I2C,
    delay: D,
    config: Config,
    is_dirty: bool,
}

impl<I2C, D> Hdc1080<I2C, D> {
    /// New HDC1080 device from an I2C peripheral with default config.
    ///
    /// Default is maximum resolution with simultaneous acquisition.
    pub fn new(i2c: I2C, delay: D) -> Self {
        Self::new_with_config(i2c, delay, Config::default())
    }

    /// New HDC1080 device from an I2C peripheral with arbitrary config.
    pub fn new_with_config(i2c: I2C, delay: D, config: Config) -> Self {
        Self {
            i2c,
            delay,
            config,
            is_dirty: true,
        }
    }

    /// Set temperature resolution.
    pub fn set_t_resolution(&mut self, t_resolution: TemperatureResolution) {
        if t_resolution != self.config.t_resolution {
            self.config.t_resolution = t_resolution;
            self.is_dirty = true;
        }
    }

    /// Set humidity resolution.
    pub fn set_h_resolution(&mut self, h_resolution: HumidityResolution) {
        if h_resolution != self.config.h_resolution {
            self.config.h_resolution = h_resolution;
            self.is_dirty = true;
        }
    }

    /// Enable or disable built-in drying heater.
    pub fn set_drying_heater(&mut self, drying_heater: impl Into<DryingHeater>) {
        let drying_heater = drying_heater.into();
        if drying_heater != self.config.drying_heater {
            self.config.drying_heater = drying_heater;
            self.is_dirty = true;
        }
    }
}

impl<I2C, D> Hdc1080<I2C, D>
where
    I2C: I2cAsync,
    D: DelayAsync,
{
    /// Fetch 16-bit register value.
    async fn fetch_register_async(&mut self, reg: Register) -> Result<u16, I2C::Error> {
        let mut buf = [0; 2];
        self.i2c
            .write_read(I2C_ADDRESS, &[reg as u8], &mut buf)
            .await?;
        Ok(u16::from_be_bytes(buf))
    }

    /// Write value to register.
    async fn write_register_async(&mut self, reg: Register, data: u16) -> Result<(), I2C::Error> {
        self.i2c
            .write(I2C_ADDRESS, &[reg as u8, (data >> 8) as u8, data as u8])
            .await
    }

    async fn read_raw_async<R: Request>(&mut self) -> Result<R, I2C::Error> {
        self.i2c.write(I2C_ADDRESS, &[R::REG as u8]).await?;
        self.delay.delay_us(R::get_delay_us(&self.config)).await;
        let mut buf: R::Buf = Default::default();
        self.i2c.read(I2C_ADDRESS, buf.as_mut()).await?;
        Ok(R::priv_from(buf))
    }

    /// Identify the device.
    ///
    /// Read manufacturer and product ID and serial number.
    pub async fn identify_async(&mut self) -> Result<Identification, I2C::Error> {
        let mut data = [0_u16; 5];
        for (i, reg) in Identification::REGISTERS.iter().enumerate() {
            data[i] = self.fetch_register_async(*reg).await?;
        }

        let cfg = self.fetch_register_async(Register::Configuration).await?;

        Ok(Identification::from_registers(data))
    }

    /// Perform device soft reset.
    pub async fn reset_async(&mut self) -> Result<(), I2C::Error> {
        self.write_register_async(Register::Configuration, RESET_COMMAND)
            .await?;
        self.delay.delay_us(RESET_DELAY).await;
        Ok(())
    }

    /// Read temperature and humidity.
    pub async fn read_async(&mut self) -> Result<(Temperature, Humidity), I2C::Error> {
        self.read_raw_async().await
    }

    /// Read temperature.
    pub async fn read_temperature_async(&mut self) -> Result<Temperature, I2C::Error> {
        self.read_raw_async().await
    }

    /// Read humidity.
    pub async fn read_humidity_async(&mut self) -> Result<Humidity, I2C::Error> {
        self.read_raw_async().await
    }
}

impl<I2C, D> Hdc1080<I2C, D>
where
    I2C: I2cBlocking,
    D: DelayBlocking,
{
    /// Fetch 16-bit register value.
    fn fetch_register_blocking(&mut self, reg: Register) -> Result<u16, I2C::Error> {
        let mut buf = [0; 2];
        self.i2c.write_read(I2C_ADDRESS, &[reg as u8], &mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    /// Write value to register.
    /// Only 8 MSBs are written since HDC1080 doesn't actually use any LSBs in register writing.
    fn write_register_blocking(&mut self, reg: Register, data: u16) -> Result<(), I2C::Error> {
        self.i2c.write(I2C_ADDRESS, &[reg as u8, (data >> 8) as u8])
    }

    /// Identify the device.
    ///
    /// Read manufacturer and product ID and serial number.
    pub fn identify_blocking(&mut self) -> Result<Identification, I2C::Error> {
        let mut data = [0_u16; 5];
        for (i, reg) in Identification::REGISTERS.iter().enumerate() {
            data[i] = self.fetch_register_blocking(*reg)?;
        }
        Ok(Identification::from_registers(data))
    }
}
