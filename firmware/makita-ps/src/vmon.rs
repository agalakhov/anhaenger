use core::sync::atomic::Ordering;
use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::task;
use embassy_stm32::{
    i2c::{mode::Master, I2c},
    mode::Async,
};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use embassy_time::{Duration, Timer};
use ina219::{
    address::{Address as Ina219Address, Pin as Ina219Pin},
    AsyncIna219,
};
use portable_atomic::AtomicI16;

pub static OUTPUT_VOLTAGE_MV: AtomicI16 = AtomicI16::new(0);
pub static OUTPUT_CURRENT_MA: AtomicI16 = AtomicI16::new(0);

const SHUNT_RESISTANCE_MILLIS: i16 = 2; // mOhm

#[task]
pub async fn process(i2c: &'static Mutex<NoopRawMutex, I2c<'static, Async, Master>>) {
    let i2c = I2cDevice::new(i2c);
    info!("Voltage monitor process started.");
    let mut output_monitor = AsyncIna219::new(
        i2c,
        Ina219Address::from_pins(Ina219Pin::Gnd, Ina219Pin::Gnd),
    )
    .await
    .expect("INA219 initialization error");
    loop {
        let out_i = output_monitor
            .shunt_voltage()
            .await
            .expect("INA219 current measurement error")
            .shunt_voltage_uv() as i16;
        let out_i = out_i / SHUNT_RESISTANCE_MILLIS;
        let out_v = output_monitor
            .bus_voltage()
            .await
            .expect("INA219 voltage measurement error")
            .voltage_mv() as i16;
        OUTPUT_VOLTAGE_MV.store(out_v, Ordering::Relaxed);
        OUTPUT_CURRENT_MA.store(out_i, Ordering::Relaxed);
        info!("Output: {} mV, {}", out_v, out_i);
        Timer::after(Duration::from_millis(100)).await;
    }
}
