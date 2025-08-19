//! Helpers for CAN messages I/O.
#![no_std]

pub mod prelude {
    pub use zerocopy::{TryFromBytes, IntoBytes, Immutable, KnownLayout};
    pub use can_messages_derive::{CanID, can_message};
}

pub use can_messages_derive::*;

/// Trait for structs that have a CAN bus identifier.
pub trait CanID {
    const ID: u16;
}

/// Extension trait for incoming CAN messages.
pub trait IncomingCan {
    fn try_decode<T: CanID>(self) -> Option<T>;
}

mod embassy {
    //use embassy_stm32;
    use super::*;

    impl IncomingCan for () {
        fn try_decode<T: CanID>(self) -> Option<T> {
            None
        }
    }
}
