#![macro_use]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use ector::*;
use embassy_time::{Duration, Timer};
use futures::future::join;

async fn test(addr: Address<Request<&'static str, &'static str>>) {
    let r = addr.request("Hello").await;
    println!("Server returned {}", r);
    Timer::after(Duration::from_secs(1)).await;
}

#[embassy_executor::main]
async fn main(_s: embassy_executor::Spawner) {
    // Example of request response
    static SERVER: ActorContext<Server> = ActorContext::new();

    let address = SERVER.address();
    let server = SERVER.mount(Server);
    let test = test(address);
    join(server, test).await;
}

pub struct Server;

impl Actor for Server {
    type Message = Request<&'static str, &'static str>;

    async fn on_mount<M>(
        &mut self,
        _: Address<Request<&'static str, &'static str>>,
        mut inbox: M,
    ) -> !
    where
        M: Inbox<Self::Message>,
    {
        println!("Server started!");

        loop {
            let motd = inbox.next().await;
            let m = motd.as_ref().clone();
            motd.reply(m).await;
        }
    }
}
