# Ector is an open source async, no-alloc actor framework for embedded devices

[![CI](https://github.com/drogue-iot/ector/actions/workflows/ci.yaml/badge.svg)](https://github.com/drogue-iot/ector/actions/workflows/ci.yaml)
[![crates.io](https://img.shields.io/crates/v/ector.svg)](https://crates.io/crates/ector)
[![docs.rs](https://docs.rs/ector/badge.svg)](https://docs.rs/ector)

Ector is an open source async, no-alloc actor framework for embedded devices. It integrates with [embassy](https://github.com/embassy-rs/embassy), the embedded async project.

## Actor System

An _actor system_ is a framework that allows for isolating state within narrow contexts, making it easier to reason about system.
Within a actor system, the primary component is an _Actor_, which represents the boundary of state usage.
Each actor has exclusive access to its own state and only communicates with other actors through message-passing.

## Example

```rust ignore
use ector::*;

/// A Counter that we wish to create an Actor for.
pub struct Counter {
    count: u32,
}

// The message our actor will handle.
pub struct Increment;

/// An Actor implements the Actor trait.
impl Actor for Counter {
    /// The Message associated type is the message types that the Actor can receive.
    type Message = Increment;

    /// An actor has to implement the on_mount method. on_mount() is invoked when the internals of an actor is ready,
    /// and the actor can begin to receive messages from an inbox.
    ///
    /// The following arguments are provided:
    /// * The address to 'self'
    /// * An inbox from which the actor can receive messages
    async fn on_mount<M>(&mut self, _: DynamicAddress<Self::Message>, mut inbox: M) -> !
    where M: Inbox<Self::Message> {
        loop {
            // Await the next message and increment the counter
            let _ = inbox.next().await;
            self.count += 1;
        }
    }
}

 /// The entry point of the application is using the embassy runtime.
 #[embassy_executor::main]
 async fn main(spawner: embassy_executor::Spawner) {

     // Mounting the Actor will spawn an embassy task
     let addr = ector::actor!(spawner, counter, Counter, Counter { count: 0 });

     // The actor address may be used in any embassy task to communicate with the actor.
     let _ = addr.notify(Increment).await;
 }
```

## Building

You can build and test the framework by running

```shell
cargo test
```

## Directory layout

* `ector` - an actor framework
* `macros` - macros used by application code to simplify

## Contributing

See the document [CONTRIBUTING.md](CONTRIBUTING.md).
