#![macro_use]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use {
    ector::{mutex::CriticalSectionRawMutex, *},
    embassy_time::{Duration, Timer},
};

#[embassy_executor::main]
async fn main(s: embassy_executor::Spawner) {
    let server_0_addr = actor!(s, server_0, Server, Server);
    let server_1_addr = actor!(s, server_1, Server, Server, CriticalSectionRawMutex);
    let server_2_addr = actor!(s, server_2, Server, Server, 2);
    let server_3_addr = actor!(s, server_3, Server, Server, CriticalSectionRawMutex, 2);

    static SERVER_4: ActorContext<Server> = ActorContext::new();
    static SERVER_5: ActorContext<Server, CriticalSectionRawMutex, 2> = ActorContext::new();
    let server_4_addr = spawn_context!(SERVER_4, s, server_4, Server, Server);
    let server_5_addr = spawn_context!(
        SERVER_5,
        s,
        server_5,
        Server,
        Server,
        CriticalSectionRawMutex,
        2
    );

    let servers = [
        server_0_addr,
        server_1_addr,
        server_2_addr,
        server_3_addr,
        server_4_addr,
        server_5_addr,
    ];
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
