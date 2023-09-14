#![macro_use]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use embassy_time::with_timeout;

use {
    ector::*,
    embassy_time::{Duration, Timer},
    futures::future::join,
};

async fn test(mut addr: DynamicRequestAddress<(), u32>) {
    let r = addr.request(()).await;
    assert_eq!(r, 1);
    println!("Server returned {}", r);
    Timer::after(Duration::from_secs(1)).await;

    match with_timeout(Duration::from_micros(1), addr.request(())).await {
        Ok(_) => panic!("Server should timeout"),
        Err(_) => println!(
            "Server timedout as expected, counter should be increased, but message skipped"
        ),
    }

    let r = addr.request(()).await;
    assert_eq!(r, 3);
    println!("Server returned {}", r);
}

#[embassy_executor::main]
async fn main(_s: embassy_executor::Spawner) {
    // Example of request response
    static SERVER: ActorContext<Server> = ActorContext::new();

    let address = request!(SERVER.dyn_address(), u32);
    let server = SERVER.mount(Server);
    let test = test(address);
    join(server, test).await;
}

pub struct Server;

impl Actor for Server {
    type Message = Request<(), u32>;

    async fn on_mount<M>(&mut self, _: DynamicAddress<Request<(), u32>>, mut inbox: M) -> !
    where
        M: Inbox<Self::Message>,
    {
        println!("Server started!");
        let mut counter = 0;

        loop {
            let req = inbox.next().await;
            counter += 1;
            // Simulate long calculation before cancel
            Timer::after(Duration::from_secs(1)).await;
            req.reply(counter).await;
        }
    }
}
