#![macro_use]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use ector::{mutex::CriticalSectionRawMutex, *};
use embassy_time::{Duration, Timer};

#[embassy_executor::main]
async fn main(s: embassy_executor::Spawner) {
    let server_1 = actor!(s, server_1, Server, Server);
    let server_2 = actor!(s, server_2, Server, Server, CriticalSectionRawMutex);
    let server_3 = actor!(s, server_3, Server, Server, 2);
    let server_4 = actor!(s, server_4, Server, Server, CriticalSectionRawMutex, 2);

    let servers = [server_1, server_2, server_3, server_4];
    loop {
        for (i, server) in servers.iter().enumerate() {
            let r = server.request("Hello".to_string()).await;
            println!("Server {} returned {}", i, r);
            Timer::after(Duration::from_secs(1)).await;
        }
    }
}

pub struct Server;

impl Actor for Server {
    type Message = Request<String, String, CriticalSectionRawMutex>;
    async fn on_mount<M>(
        &mut self,
        _: Address<Request<String, String, CriticalSectionRawMutex>>,
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
