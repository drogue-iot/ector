#![macro_use]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use ector::mutex::CriticalSectionRawMutex;

use {
    ector::*,
    embassy_time::{Duration, Timer},
};
// Address<Request<String, String>, CriticalSectionRawMutex, 1>
#[embassy_executor::task]
async fn send_task(
    mut address: RequestManager<
        Address<Request<String, String>, CriticalSectionRawMutex, 1>,
        String,
        String,
        CriticalSectionRawMutex,
    >,
) {
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
    let request_manager = req!(server_addr, String, CriticalSectionRawMutex);
    send_spawner.spawn(send_task(request_manager)).unwrap();
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
