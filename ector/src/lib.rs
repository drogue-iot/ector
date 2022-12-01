#![macro_use]
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(dead_code)]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![feature(impl_trait_projections)]
#![doc = include_str!("../README.md")]
pub(crate) mod fmt;

mod actor;

pub use actor::*;

/// Spawn an actor given a spawner and the actors name, type and instance.
#[macro_export]
macro_rules! actor {
    ($spawner:ident, $name:ident, $ty:ty, $instance:expr) => {{
        #[embassy_executor::task]
        async fn spawn_$name(a: &'static ::ector::ActorContext<$ty>) {
            a.await
        }

        static $name: ::ector::ActorContext<$ty> = ::ector::ActorContext::new();
        let _ctx = $name.mount($instance);
        let address = $name.address();
        $spawner.spawn($name_task(_ctx)).unwrap();
        address
    }};
}
