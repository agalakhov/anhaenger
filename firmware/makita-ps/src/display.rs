use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306Async};

use core::fmt::Write;
use core::sync::atomic::Ordering;
use defmt::error;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::task;
use embassy_stm32::{
    i2c::{mode::Master, I2c},
    mode::Async,
};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use embassy_time::Timer;
use heapless::String;

use crate::{
    adc::BATTERY_VOLTAGE_MV,
    vmon::{OUTPUT_CURRENT_MA, OUTPUT_VOLTAGE_MV},
};

#[task]
pub async fn process(i2c: &'static Mutex<NoopRawMutex, I2c<'static, Async, Master>>) {
    let i2c = I2cDevice::new(i2c);
    let iface = I2CDisplayInterface::new(i2c);

    let mut display = Ssd1306Async::new(iface, DisplaySize128x32, DisplayRotation::Rotate180)
        .into_terminal_mode();

    let mut init_count = 10;
    loop {
        if display.init().await.is_ok() {
            break;
        }
        init_count -= 1;
        if init_count == 0 {
            error!("Display initialization error");
            return;
        }
        Timer::after_millis(10).await;
    }

    let _ = display.clear().await;

    loop {
        let _ = display.set_position(0, 0).await;
        let batt_voltage = BATTERY_VOLTAGE_MV.load(Ordering::Relaxed);
        let output_voltage = OUTPUT_VOLTAGE_MV.load(Ordering::Relaxed);
        let output_current = OUTPUT_CURRENT_MA.load(Ordering::Relaxed);
        let power = output_voltage as i32 * output_current as i32 / 1000;

        let mut s = String::<128>::new();
        let _ = write!(s, "Bat: {batt_voltage:>5} mV\nOut: {output_voltage:>5} mV\nCur: {output_current:>5} mA\nP: {power:>7} mW");
        let _ = display.write_str(&s).await;

        Timer::after_millis(300).await;
    }
}
