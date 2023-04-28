#![macro_use]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use {
    ector::*,
    embassy_time::{Duration, Ticker},
    futures::{
        future::{join, select, Either},
        pin_mut,
    },
};

#[embassy_executor::main]
async fn main(_s: embassy_executor::Spawner) {
    // Example of circular references
    static PINGER: ActorContext<Pinger> = ActorContext::new();
    static PONGER: ActorContext<Ponger> = ActorContext::new();

    let pinger_addr = PINGER.dyn_address();
    let ponger_addr = PONGER.dyn_address();

    let pinger = PINGER.mount(Pinger(ponger_addr));
    let ponger = PONGER.mount(Ponger(pinger_addr));
    join(pinger, ponger).await;
}

#[derive(Debug)]
pub struct Ping;

#[derive(Debug)]
pub struct Pong;

pub struct Pinger(DynamicAddress<Ping>);
pub struct Ponger(DynamicAddress<Pong>);

impl Actor for Pinger {
    type Message = Pong;

    async fn on_mount<M>(&mut self, _: DynamicAddress<Pong>, mut inbox: M) -> !
    where
        M: Inbox<Self::Message>,
    {
        println!("Pinger started!");

        let mut ticker = Ticker::every(Duration::from_secs(2));
        // We need to store the pinger to send a message back
        loop {
            let next = inbox.next();
            let tick = ticker.next();

            pin_mut!(next);
            pin_mut!(tick);

            // Send a ping every 10 seconds
            match select(next, tick).await {
                Either::Left((m, _)) => {
                    println!("{:?}", m);
                }
                Either::Right((_, _)) => {
                    self.0.notify(Ping).await;
                }
            }
        }
    }
}

impl Actor for Ponger {
    type Message = Ping;
    async fn on_mount<M>(&mut self, _: DynamicAddress<Ping>, mut inbox: M) -> !
    where
        M: Inbox<Self::Message>,
    {
        println!("Ponger started!");

        loop {
            // Send a ping every 10 seconds
            let m = inbox.next().await;
            println!("{:?}", m);
            self.0.notify(Pong).await;
        }
    }
}
