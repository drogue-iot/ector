//! Example where cancellation will cause a panic

#![macro_use]
#![allow(incomplete_features)]

use {
    ector::*,
    embassy_time::{Duration, Timer},
    futures::future::{join, select},
};

async fn test(addr: DynamicAddress<Request<&'static str, &'static str>>) {
    let req = core::pin::pin!(addr.request("Hello"));
    let timeout = Timer::after(Duration::from_secs(1));
    select(req, timeout).await;
    println!("Timeout");
}

#[embassy_executor::main]
async fn main(_s: embassy_executor::Spawner) {
    // Example of request response
    static SERVER: ActorContext<Server> = ActorContext::new();

    let address = SERVER.dyn_address();
    let server = SERVER.mount(Server);
    let test = test(address);
    join(server, test).await;
}

pub struct Server;

impl Actor for Server {
    type Message = Request<&'static str, &'static str>;

    async fn on_mount<M>(
        &mut self,
        _: DynamicAddress<Request<&'static str, &'static str>>,
        mut inbox: M,
    ) -> !
    where
        M: Inbox<Self::Message>,
    {
        println!("Server started!");

        loop {
            // We don't reply, since we cancel
            inbox.next().await;
        }
    }
}
