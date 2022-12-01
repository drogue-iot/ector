#![macro_use]
#![feature(async_fn_in_trait)]
#![feature(type_alias_impl_trait)]

use {
    ector::*,
    embassy_time::{Duration, Timer},
};

static SERVER: ActorContext<Server> = ActorContext::new();

#[embassy_executor::main]
async fn main(s: embassy_executor::Spawner) {
    // Example of request response
    s.spawn(actor_task(Server)).unwrap();
    let server = SERVER.address();

    loop {
        let r = server.request("Hello").await;
        println!("Server returned {}", r);
        Timer::after(Duration::from_secs(1)).await;
    }
}

#[embassy_executor::task]
async fn actor_task(a: Server) {
    SERVER.mount(a).await;
}

pub struct Server;

impl Actor for Server {
    type Message<'m> = Request<&'static str, &'static str>;
    async fn on_mount<'m, M>(
        &'m mut self,
        _: Address<Request<&'static str, &'static str>>,
        mut inbox: M,
    ) -> !
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
