#![macro_use]
#![feature(type_alias_impl_trait)]

use {
    ector::*,
    embassy_time::{Duration, Timer},
};

#[embassy_executor::main]
async fn main(s: embassy_executor::Spawner) {
    // Example of request response
    static SERVER: ActorContext<Server> = ActorContext::new();

    let server = SERVER.mount(s, Server);

    loop {
        let r = server.request("Hello").await;
        println!("Server returned {}", r);
        Timer::after(Duration::from_secs(1)).await;
    }
}

pub struct Server;

#[actor]
impl Actor for Server {
    type Message<'m> = Request<&'static str, &'static str>;
    async fn on_mount<M>(&mut self, _: Address<Request<&'static str, &'static str>>, mut inbox: M)
    where
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
