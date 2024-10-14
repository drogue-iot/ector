#![macro_use]
#![allow(incomplete_features)]

use ector::mutex::CriticalSectionRawMutex;

use {
    ector::*,
    embassy_time::{Duration, Timer},
};

#[embassy_executor::task]
async fn send_task(address: Address<Request<String, String>, CriticalSectionRawMutex, 1>) {
    loop {
        let r = address.request("Hello".to_string()).await;
        println!("Server returned {}", r);
        Timer::after(Duration::from_secs(1)).await;
    }
}

#[embassy_executor::main]
async fn main(s: embassy_executor::Spawner) {
    let send_spawner = s.make_send();
    //  CriticalSectionRawMutex makes the address `Send`
    let server_addr = actor!(s, server_0, Server, Server, CriticalSectionRawMutex);
    send_spawner.spawn(send_task(server_addr)).unwrap();
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
