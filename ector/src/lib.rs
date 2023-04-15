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

#[cfg(all(feature = "std", feature = "test-util"))]
pub mod testutil;

/// Spawn an actor given a spawner and the actors name, type and instance.
#[macro_export]
macro_rules! spawn_actor {
    ($spawner:ident, $name:ident, $ty:ty, $instance:expr) => {{
        static $name: ::ector::ActorContext<$ty> = ::ector::ActorContext::new();
        $name.mount($spawner, $instance)
    }};
}
