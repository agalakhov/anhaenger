//! ADC driver reading battery voltage and control pin voltage

use array_macro::array;
use core::{
    ptr,
    sync::atomic::{AtomicI16, AtomicU16, Ordering},
};
use defmt::{debug, info};
use embassy_executor::task;
use embassy_stm32::{
    adc::{resolution_to_max_count, Adc, AnyAdcChannel, Resolution, SampleTime, VDDA_CALIB_MV},
    gpio::Output,
    peripherals::ADC1,
};
use embassy_time::Timer;

const VOLT_FACTOR: u32 = 10;
const RESOLUTION: Resolution = Resolution::BITS12;

pub static CPU_TEMPERATURE: AtomicI16 = AtomicI16::new(0);
pub static CURRENTS: [AtomicU16; 4] = array![_ => AtomicU16::new(0); 4];

fn get_vref_cal() -> u32 {
    unsafe {
        // DocID025832 Rev. 5
        ptr::read_volatile(0x1FFF_F7BA as *const u16) as u32
    }
}

fn get_ts_cal() -> (i32, i32) {
    unsafe {
        // DocID025832 Rev. 5
        (
            ptr::read_volatile(0x1FFF_F7B8 as *const u16) as i32,
            ptr::read_volatile(0x1FFF_F7C2 as *const u16) as i32,
        )
    }
}

#[task]
pub async fn process(
    mut adc: Adc<'static, ADC1>,
    mut pin_sense: AnyAdcChannel<ADC1>,
    mut selector: [Output<'static>; 2],
) {
    let vref_cal = get_vref_cal();
    let (t30_cal, t110_cal) = get_ts_cal();
    adc.set_resolution(RESOLUTION);
    adc.set_sample_time(SampleTime::CYCLES239_5);

    let mut idx = 0;

    let mut reference = adc.enable_vref();
    let mut tempsensor = adc.enable_temperature();
    info!("ADC calibration value = {}", vref_cal);
    info!("T calibration values = {}, {}", t30_cal, t110_cal);
    let max = resolution_to_max_count(RESOLUTION);
    loop {
        for i in 0..selector.len() {
            selector[i].set_level(((idx >> i) & 1 == 1).into());
        }
        let settle_timer = Timer::after_micros(60);

        let temperature = adc.read(&mut tempsensor).await;
        let vref = adc.read(&mut reference).await;
        settle_timer.await;
        let voltage = adc.read(&mut pin_sense).await;

        // RM0091 13.8 Calculating the actual VDDA voltage using the internal reference voltage
        // V_DDA = 3.3 V x VREFINT_CAL / VREFINT_DATA
        let vdda = (vref_cal * VDDA_CALIB_MV) / vref as u32;

        // RM0091 13.8 Reading the temperature
        // T = (110 °C - 30 °C) / (TS_CAL2 - TS_CAL1) × (TS_DATA - TS_CAL1) + 30 °C
        let ts = temperature as i32 * 3300 / vdda as i32;
        let temperature = ((ts - t30_cal) * (110 - 30) / (t110_cal - t30_cal) + 30) as i16;

        let sense_voltage_mv = (voltage as u32 * vdda / max * VOLT_FACTOR) as u16;
        let current_ma = sense_voltage_mv * 2;

        //        debug!("Ch[{}] = {} mA", idx, current_ma);

        CURRENTS[idx].store(current_ma, Ordering::Relaxed);
        CPU_TEMPERATURE.store(temperature, Ordering::Relaxed);

        Timer::after_millis(100).await;
        idx = (idx + 1) % CURRENTS.len();
    }
}
