#![macro_use]
#![feature(type_alias_impl_trait)]

use ector::{mutex::CriticalSectionRawMutex, *};
use embassy_time::{Duration, Timer};

#[embassy_executor::main]
async fn main(s: embassy_executor::Spawner) {
    let server_1 = spawn_actor!(s, SERVER_1, Server, Server);
    let server_2 = spawn_actor!(s, SERVER_2, Server, Server, CriticalSectionRawMutex);
    let server_3 = spawn_actor!(s, SERVER_3, Server, Server, 2);
    let server_4 = spawn_actor!(s, SERVER_4, Server, Server, CriticalSectionRawMutex, 2);

    let servers = [server_1, server_2, server_3, server_4];
    loop {
        for (i, server) in servers.iter().enumerate() {
            let r = server.request("Hello").await;
            println!("Server {} returned {}", i, r);
            Timer::after(Duration::from_secs(1)).await;
        }
    }
}

pub struct Server;

#[actor]
impl Actor for Server {
    type Message<'m> = Request<&'static str, &'static str, CriticalSectionRawMutex>;
    async fn on_mount<M>(
        &mut self,
        _: Address<Request<&'static str, &'static str, CriticalSectionRawMutex>>,
        mut inbox: M,
    ) where
        M: Inbox<Self::Message<'m>> + 'm,
    {
        println!("Server started!");

        loop {
            let motd = inbox.next().await;
            let m = motd.as_ref().clone();
            motd.reply(m).await;
        }
    }
}
