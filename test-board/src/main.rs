#![feature(impl_trait_in_assoc_type)]
#![no_std]
#![no_main]

use {defmt_rtt as _, panic_probe as _};

use defmt::{info, Debug2Format};
use embassy_executor::{main, task, Spawner};
use embassy_futures::{join::join, select::select};
use embassy_stm32::{
    bind_interrupts,
    can::{self as stm32_can, Can, frame::Frame, Fifo, filter::Mask32, Id, StandardId, CanTx},
    gpio::{Level, Output, Speed, Pull},
    i2c::{self, mode::Master, I2c, Config as I2cConfig},
    mode::Async,
    peripherals,
    time::khz,
    Config as DeviceConfig,
    pac,
    exti::ExtiInput,
};
use embassy_time::{Duration, Timer};
use can_messages::{BITRATE, CanId, BatteryData};
use zerocopy::TryFromBytes;

bind_interrupts!(struct Irqs {
    I2C1 => i2c::EventInterruptHandler<peripherals::I2C1>, i2c::ErrorInterruptHandler<peripherals::I2C1>;
    CEC_CAN => stm32_can::Rx0InterruptHandler<peripherals::CAN>, stm32_can::Rx1InterruptHandler<peripherals::CAN>,
               stm32_can::TxInterruptHandler<peripherals::CAN>, stm32_can::SceInterruptHandler<peripherals::CAN>;
});

#[task]
async fn send_poweroff(mut tx: CanTx<'static>, mut btn: ExtiInput<'static>) {
    loop {
        btn.wait_for_falling_edge().await;
        let frame = Frame::new_standard(CanId::POWEROFF.into(), &[]).unwrap();
        tx.write(&frame).await;
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
    let _i2c = I2c::new(
        dev.I2C1,
        scl,
        sda,
        Irqs,
        dev.DMA1_CH2,
        dev.DMA1_CH3,
        {
            let mut cfg = I2cConfig::default();
            cfg.frequency = khz(400);
            cfg
        }
    );

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
            if *msg.frame.id() == Id::Standard(StandardId::new(CanId::BATTERY as u16).unwrap()) {
                let batt = BatteryData::try_ref_from_bytes(msg.frame.data());

                info!("CAN battery: {}", Debug2Format(&batt));
            } else {
                info!("CAN message received: {}", Debug2Format(&msg));
            }
        }
    }
}
