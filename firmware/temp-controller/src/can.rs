use embassy_stm32::can::{Can, CanRx, CanTx};
use embassy_executor::task;
use defmt::{info, Debug2Format};
use embassy_time::Timer;
use embassy_futures::join::join;
use can_messages::{prelude::*, BITRATE, CoolBox};
use crate::temperature::TEMPERATURE;
use core::sync::atomic::Ordering;

#[task]
pub async fn process(mut can: Can<'static>) {
    can.set_bitrate(BITRATE);
    can.set_tx_fifo_scheduling(true);
    can.enable().await;
    info!("CAN initialized.");
    let (tx, rx) = can.split();
    join(transmit(tx), receive(rx)).await;
}

async fn receive(mut rx: CanRx<'static>) {
    loop {
        let _ = rx.read().await;
    }
}

async fn transmit(mut tx: CanTx<'static>) {
    let mut mailbox = None;
    loop {
        let box_temperature_deg10 = TEMPERATURE.load(Ordering::Relaxed);

        let data = CoolBox {
            box_temperature_deg10,
        };

        if let Some(frame) = data.try_encode() {
            if let Some(mbox) = mailbox.take() {
                let r = tx.abort(mbox);
                info!("CAN send: {}", r);
            }
            if let Ok(wr) = tx.try_write(&frame) {
                mailbox = Some(wr.mailbox());
            } else {
                info!("CAN send fail");
            }
        }

        Timer::after_millis(100).await;
    }
}
