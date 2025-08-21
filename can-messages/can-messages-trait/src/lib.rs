//! Helpers for CAN messages I/O.
#![no_std]

pub mod prelude {
    pub use zerocopy::{TryFromBytes, IntoBytes, Immutable, KnownLayout};
    pub use can_messages_derive::can_message;
    pub use super::{CanMessage, IncomingCan, OutgoingCan, CanParseable, can_variant};
}

pub use can_messages_derive::*;

use zerocopy::{TryFromBytes, IntoBytes, Immutable, KnownLayout};

/// Trait for structs that have a CAN bus identifier.
pub trait CanMessage: TryFromBytes + IntoBytes + Immutable + KnownLayout {
    const ID: u16;
}

/// Extension trait for anything that is CAN-parseable.
pub trait CanParseable {
    fn id_matches<T: CanMessage>(&self) -> bool;
    fn as_bytes(&self) -> &[u8];
}

/// Extension trait for incoming CAN messages.
pub trait IncomingCan {
    fn try_decode<T: CanMessage>(&self) -> Option<&T>;
}

impl<C> IncomingCan for C
where
    C: CanParseable,
{
    fn try_decode<T: CanMessage>(&self) -> Option<&T> {
        if self.id_matches::<T>() {
            T::try_ref_from_bytes(self.as_bytes()).ok()
        } else {
            None
        }
    }
}

/// Extension trait for outfoing CAN messages.
pub trait OutgoingCan<T> {
    fn try_encode(&self) -> Option<T>;
}

#[macro_export]
macro_rules! can_variant {
    ($name:ident { $( $n:ident ( $i:path ) ),* $(,)? } ) => {
        enum $name {
            $(
                $n($i)
            ),*
        }

        impl Default for $name
        where
            $(
                $i: Default
            ),*
        {
            fn default() -> Self {
                unimplemented!()
            }
        }
    }
}

#[cfg(feature = "embassy")]
mod embassy {
    use embassy_stm32::can::{Id, StandardId, frame::{Frame, Envelope}};
    use crate::prelude::*;

    impl CanParseable for Frame {
        fn id_matches<T: CanMessage>(&self) -> bool {
            StandardId::new(T::ID)
                .map(|id| *self.id() == Id::Standard(id))
                .unwrap_or(false)
        }
        fn as_bytes(&self) -> &[u8] {
            self.data()
        }
    }

    impl CanParseable for Envelope {
        fn id_matches<T: CanMessage>(&self) -> bool {
            self.frame.id_matches::<T>()
        }
        fn as_bytes(&self) -> &[u8] {
            self.frame.as_bytes()
        }
    }

    impl<T> OutgoingCan<Frame> for T
    where
        T: CanMessage,
    {
        fn try_encode(&self) -> Option<Frame> {
            Frame::new_standard(Self::ID.into(), self.as_bytes()).ok()
        }
    }
}
