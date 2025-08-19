use embassy_stm32::can::Can;
use embassy_executor::task;
use defmt::{info, Debug2Format};

#[task]
pub async fn process(mut can: Can<'static>) {
    can.enable().await;
    info!("CAN initialized.");
    let (mut tx, mut rx) = can.split();
    loop {
        if let Ok(msg) = rx.read().await {
            info!("CAN message: {}", Debug2Format(&msg));
        }
    }
}
