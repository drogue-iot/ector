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
///
/// actor! is a macro that simplifies the process of spawning an actor. It creates a new ActorContext and spawns the actor with the given spawner.
///
/// # Arguments
///
/// `actor!(spawner, name, type, instance, [mutex type], [queue size]);`
///
/// |             |                                                                                                        |
/// | ----------- | ------------------------------------------------------------------------------------------------------ |
/// | spawner     | The spawner to use to spawn the actor (i.e. embassy_executor::Spawner)                                 |
/// | name        | The name of the actor, used to generate the task name                                                  |
/// | type        | The type of the actor, must implement the Actor trait                                                  |
/// | instance    | The instance of the actor to spawn                                                                     |
/// | mutex type  | The type of mutex to use for the actor. (defaults to embassy_sync::blocking_mutex::raw::NoopRawMutex)  |
/// | queue size  | The size of the actor's message queue. (defaults to 1)                                                 |
///
///
/// # Example
///
/// ```rust ignore
/// use ector::*;
///
/// type Addr = DynamicAddress<Request<String, String>>;
///
/// struct Server;
///
/// impl Actor for Server {
///     type Message = Request<String, String>;
///     async fn on_mount<M>(&mut self, _: Addr, mut inbox: M) -> !
///     where
///        M: Inbox<Self::Message>,
///    {
///        println!("Server started!");
///
///        loop {
///            let motd = inbox.next().await;
///            let m = motd.as_ref().clone();
///            motd.reply(m).await;
///        }
///    }
/// }
///
/// #[embassy_executor::main]
/// async fn main(s: embassy_executor::Spawner) {
///     let server_addr: Addr = actor!(s, server, Server, Server).into();
///     loop {
///          let r = server_addr.request("Hello".to_string()).await;
///          println!("Server returned {}", r);
///     }
/// }
///
/// ```
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

    ($context:ident, $spawner:ident, $name:ident, $ty:ty, $instance:expr, $queue_size:literal) => {{
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

    ($context:ident, $spawner:ident, $name:ident, $ty:ty, $instance:expr, $mutex:ty, $queue_size:literal) => {{
        #[embassy_executor::task]
        async fn $name(a: &'static ::ector::ActorContext<$ty, $mutex, $queue_size>, instance: $ty) {
            a.mount(instance).await
        }

        let address = $context.address();
        $spawner.spawn($name(&$context, $instance)).unwrap();
        address
    }};
}
