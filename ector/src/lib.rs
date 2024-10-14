#![macro_use]
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(dead_code)]
#![allow(async_fn_in_trait)]
#![doc = include_str!("../README.md")]

pub(crate) mod fmt;

mod actor;
mod drop;
pub use {actor::*, ector_macros};

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

    ($spawner:ident, $name:ident, $ty:ty, $instance:expr, $queue_size:expr) => {{
        ::ector::actor!(
            $spawner,
            $name,
            $ty,
            $instance,
            ::ector::mutex::NoopRawMutex,
            $queue_size
        )
    }};

    ($spawner:ident, $name:ident, $ty:ty, $instance:expr, $mutex:ty, $queue_size:expr) => {{
        static CONTEXT: ::ector::ActorContext<$ty, $mutex, $queue_size> =
            ::ector::ActorContext::new();
        ::ector::spawn_context!(
            CONTEXT,
            $spawner,
            $name,
            $ty,
            $instance,
            $mutex,
            $queue_size
        )
    }};
}

/// Spawn an ActorContext for a given spawner
#[macro_export]
macro_rules! spawn_context {
    ($context:ident, $spawner:ident, $name:ident, $ty:ty, $instance:expr) => {{
        ::ector::spawn_context!(
            $context,
            $spawner,
            $name,
            $ty,
            $instance,
            ::ector::mutex::NoopRawMutex,
            1
        )
    }};

    ($context:ident, $spawner:ident, $name:ident, $ty:ty, $instance:expr, $mutex:ty) => {{
        ::ector::spawn_context!($context, $spawner, $name, $ty, $instance, $mutex, 1)
    }};

    ($context:ident, $spawner:ident, $name:ident, $ty:ty, $instance:expr, $queue_size:expr) => {{
        ::ector::spawn_context!(
            $context,
            $spawner,
            $name,
            $ty,
            $instance,
            ::ector::mutex::NoopRawMutex,
            $queue_size
        )
    }};

    ($context:ident, $spawner:ident, $name:ident, $ty:ty, $instance:expr, $mutex:ty, $queue_size:expr) => {{
        #[embassy_executor::task]
        async fn $name(a: &'static ::ector::ActorContext<$ty, $mutex, $queue_size>, instance: $ty) {
            a.mount(instance).await
        }

        let address = $context.address();
        $spawner.spawn($name(&$context, $instance)).unwrap();
        address
    }};
}
