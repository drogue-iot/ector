#![macro_use]
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(dead_code)]
#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]
//! Ector is an open source async, no-alloc actor framework for embedded devices. It integrates with [embassy](https://github.com/embassy-rs/embassy), the embedded async project.
//!
//! # Actor System
//!
//! An _actor system_ is a framework that allows for isolating state within narrow contexts, making it easier to reason about system.
//! Within a actor system, the primary component is an _Actor_, which represents the boundary of state usage.
//! Each actor has exclusive access to its own state and only communicates with other actors through message-passing.
//!
//! # Example
//!
//! ```
//! #![macro_use]
//! #![feature(generic_associated_types)]
//! #![feature(type_alias_impl_trait)]
//!
//! # use ector::*;
//!
//! /// A Counter that we wish to create an Actor for.
//! pub struct Counter {
//!     count: u32,
//! }
//!
//! // The message our actor will handle.
//! pub struct Increment;
//!
//! /// An Actor implements the Actor trait.
//! #[ector::actor]
//! impl Actor for Counter {
//!     /// The Message associated type is the message types that the Actor can receive.
//!     type Message<'m> = Increment;
//!
//!     /// An actor has to implement the on_mount method. on_mount() is invoked when the internals of an actor is ready,
//!     /// and the actor can begin to receive messages from an inbox.
//!     ///
//!     /// The following arguments are provided:
//!     /// * The address to 'self'
//!     /// * An inbox from which the actor can receive messages
//!     async fn on_mount<M>(&mut self, _: Address<Self::Message<'m>>, mut inbox: M)
//!         where M: Inbox<Self::Message<'m>> {
//!     {
//!         loop {
//!             // Await the next message and increment the counter
//!             let _ = inbox.next().await;
//!             self.count += 1;
//!         }
//!     }
//! }
//!
//! /// The entry point of the application is using the embassy runtime.
//! #[embassy::main]
//! async fn main(spawner: embassy::executor::Spawner) {
//!
//!     // Mounting the Actor will spawn an embassy task
//!     let addr = ector::spawn_actor!(spawner, COUNTER, Counter, Counter { count  0 });
//!
//!     // The actor address may be used in any embassy task to communicate with the actor.
//!     let _ = addr.notify(Increment).await;
//! }
//!```
//!

pub(crate) mod fmt;

mod actor;
mod device;

pub use actor::*;
pub use device::*;
pub use ector_macros::*;

#[cfg(feature = "std")]
pub mod testutil;

/// Spawn an actor given a spawner and the actors name, type and instance.
#[macro_export]
macro_rules! spawn_actor {
    ($spawner:ident, $name:ident, $ty:ty, $instance:expr) => {{
        static $name: ::ector::ActorContext<$ty> = ::ector::ActorContext::new();
        $name.mount($spawner, $instance)
    }};
}
