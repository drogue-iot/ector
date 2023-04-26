#![macro_use]
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(dead_code)]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]
#![doc = include_str!("../README.md")]
pub(crate) mod fmt;

mod actor;
pub use {actor::*, ector_macros::*};

// Reexport mutex types
pub mod mutex {
    pub use embassy_sync::blocking_mutex::raw::*;
}

#[cfg(feature = "test-utils")]
pub mod testutils;

/// Spawn an actor given a spawner and the actors name, type and instance.
#[macro_export]
macro_rules! actor {
    ($spawner:ident, $name:ident, $ty:ty, $instance:expr) => {{
        ::ector::actor!(
            $spawner,
            $name,
            $ty,
            $instance,
            ::ector::mutex::NoopRawMutex,
            1
        )
    }};

    ($spawner:ident, $name:ident, $ty:ty, $instance:expr, $mutex:ty) => {{
        ::ector::actor!($spawner, $name, $ty, $instance, $mutex, 1)
    }};

    ($spawner:ident, $name:ident, $ty:ty, $instance:expr, $queue_size:literal) => {{
        ::ector::actor!(
            $spawner,
            $name,
            $ty,
            $instance,
            ::ector::mutex::NoopRawMutex,
            $queue_size
        )
    }};

    ($spawner:ident, $name:ident, $ty:ty, $instance:expr, $mutex:ty, $queue_size:literal) => {{
        #[embassy_executor::task]
        async fn $name(a: &'static ::ector::ActorContext<$ty, $mutex, $queue_size>, instance: $ty) {
            a.mount(instance).await
        }

        static CONTEXT: ::ector::ActorContext<$ty, $mutex, $queue_size> =
            ::ector::ActorContext::new();
        let address = CONTEXT.address();
        $spawner.spawn($name(&CONTEXT, $instance)).unwrap();
        address
    }};
}
