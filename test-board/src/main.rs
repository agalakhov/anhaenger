#![feature(impl_trait_in_assoc_type)]
#![no_std]
#![no_main]

use {defmt_rtt as _, panic_probe as _};

use defmt::{info, Debug2Format};
use embassy_executor::{main, task, Spawner};
use embassy_futures::{join::join, select::select};
use embassy_stm32::{
    bind_interrupts,
    can::{self as stm32_can, Can, Fifo, filter::Mask32, Id, StandardId, CanTx},
    gpio::{Level, Output, Speed, Pull},
    i2c::{self, mode::Master, I2c, Config as I2cConfig},
    mode::Async,
    peripherals,
    time::khz,
    Config as DeviceConfig,
    pac,
    exti::ExtiInput,
};
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306Async};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use static_cell::StaticCell;
use embassy_time::{Duration, Timer};
use can_messages::{prelude::*, BITRATE, PowerOff, BatteryData, CoolBox};
use heapless::String;
use core::fmt::Write;

bind_interrupts!(struct Irqs {
    I2C1 => i2c::EventInterruptHandler<peripherals::I2C1>, i2c::ErrorInterruptHandler<peripherals::I2C1>;
    CEC_CAN => stm32_can::Rx0InterruptHandler<peripherals::CAN>, stm32_can::Rx1InterruptHandler<peripherals::CAN>,
               stm32_can::TxInterruptHandler<peripherals::CAN>, stm32_can::SceInterruptHandler<peripherals::CAN>;
});

#[task]
async fn send_poweroff(mut tx: CanTx<'static>, mut btn: ExtiInput<'static>) {
    loop {
        btn.wait_for_falling_edge().await;
        tx.write(&PowerOff.try_encode().unwrap()).await;
    }
}

#[main]
async fn main(spawner: Spawner) {
    // HSI oscillator 12 MHz, 64 MHz system frequency
    let mut config = DeviceConfig::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.hsi = true;
        config.rcc.hse = None;
        config.rcc.pll = Some(Pll {
            src: PllSource::HSI,
            prediv: PllPreDiv::DIV1,
            mul: PllMul::MUL6,
        });
        config.rcc.sys = Sysclk::PLL1_P;
        config.rcc.ahb_pre = AHBPrescaler::DIV1;
        config.rcc.apb1_pre = APBPrescaler::DIV1;
    }
    let dev = embassy_stm32::init(config);

    // Reconfigure pins for CAN bus
    pac::SYSCFG.cfgr1().modify(|w| w.set_pa11_pa12_rmp(true));

    // Button
    let btn = ExtiInput::new(dev.PA6, dev.EXTI6, Pull::Up);

    // I²C bus
    let scl = dev.PF1;
    let sda = dev.PF0;
    let i2c = I2c::new(
        dev.I2C1,
        scl,
        sda,
        Irqs,
        dev.DMA1_CH2,
        dev.DMA1_CH3,
        {
            let mut cfg = I2cConfig::default();
            cfg.frequency = khz(400);
            cfg.sda_pullup = true;
            cfg.scl_pullup = true;
            cfg
        }
    );

    static I2C_BUS: StaticCell<Mutex<NoopRawMutex, I2c<'_, Async, Master>>> = StaticCell::new();
    let i2c = Mutex::new(i2c);
    let i2c = I2C_BUS.init(i2c);

    let i2c = I2cDevice::new(i2c);
    let iface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306Async::new(iface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_terminal_mode();

    for _ in 0..10 {
        let r = display.init().await;
        if r.is_ok() {
            break;
        }
        Timer::after_millis(10).await;
    }

    let _ = display.clear().await;
    let _ = display.write_str("It works!").await;

    let mut can = Can::new(dev.CAN, dev.PA11, dev.PA12, Irqs);
    can.set_bitrate(BITRATE);
    can.set_tx_fifo_scheduling(true);
    can.enable().await;
    info!("CAN initialized.");
    let (tx, mut rx) = can.split();

    rx.modify_filters()
        .enable_bank(0, Fifo::Fifo0, Mask32::accept_all());

    spawner.spawn(send_poweroff(tx, btn)).unwrap();

    info!("System startup");
    loop {
        if let Ok(msg) = rx.read().await {
            if let Some(batt) = msg.try_decode::<BatteryData>() {
                info!("CAN battery: {}", Debug2Format(&batt));
                let _ = display.set_position(0, 0).await;
                let mut buf = String::<128>::new();
                let _ = write!(&mut buf, "Bat: {:>5} mV", batt.battery_voltage_mv);
                let _ = display.write_str(&buf).await;
            } else if let Some(cob) = msg.try_decode::<CoolBox>() {
                info!("CAN coolbox: {}", Debug2Format(&cob));
                let _ = display.set_position(0, 1).await;
                let mut buf = String::<128>::new();
                let _ = write!(&mut buf, "Temp: {:>5} /10C", cob.box_temperature_deg10);
                let _ = display.write_str(&buf).await;
            } else {
                info!("CAN message received: {}", Debug2Format(&msg));
            }
        }
    }
}
