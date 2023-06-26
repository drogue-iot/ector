#![macro_use]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use {
    ector::{mutex::NoopRawMutex, *},
    embassy_time::{Duration, Timer},
};

#[embassy_executor::main]
async fn main(s: embassy_executor::Spawner) {
    let server_0_addr: DynamicAddress<Request<String, String>> =
        actor!(s, server_0, Server, Server).into();

    let server_1_addr = actor!(s, server_1, Server, Server, NoopRawMutex).into();
    let server_2_addr = actor!(s, server_2, Server, Server, 2).into();
    let server_3_addr = actor!(s, server_3, Server, Server, NoopRawMutex, 2).into();

    static SERVER_4: ActorContext<Server> = ActorContext::new();
    static SERVER_5: ActorContext<Server, NoopRawMutex, 2> = ActorContext::new();
    let server_4_addr = spawn_context!(SERVER_4, s, server_4, Server, Server).into();
    let server_5_addr =
        spawn_context!(SERVER_5, s, server_5, Server, Server, NoopRawMutex, 2).into();

    // Adding support for request
    let server_0_addr = request!(server_0_addr, String);
    let server_1_addr = request!(server_1_addr, String);
    let server_2_addr = request!(server_2_addr, String);
    let server_3_addr = request!(server_3_addr, String);
    let server_4_addr = request!(server_4_addr, String);
    let server_5_addr = request!(server_5_addr, String);

    // Array of DynamicAddress
    let mut servers = [
        server_0_addr,
        server_1_addr,
        server_2_addr,
        server_3_addr,
        server_4_addr,
        server_5_addr,
    ];
    loop {
        for (i, server) in servers.iter_mut().enumerate() {
            let r = server.request("Hello".to_string()).await;
            println!("Server {} returned {}", i, r);
            Timer::after(Duration::from_secs(1)).await;
        }
    }
}

pub struct Server;

impl Actor for Server {
    type Message = Request<String, String>;
    async fn on_mount<M>(&mut self, _: DynamicAddress<Request<String, String>>, mut inbox: M) -> !
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
