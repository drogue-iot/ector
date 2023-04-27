#![macro_use]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

#[cfg(feature = "std")]
mod tests {
    use {
        ector::*,
        embassy_executor::Spawner,
        std::{
            sync::{
                atomic::{AtomicU32, Ordering},
                mpsc,
            },
            thread,
            time::Duration,
        },
    };

    static INITIALIZED: AtomicU32 = AtomicU32::new(0);

    #[test]
    fn test_device_setup() {
        pub struct MyActor {
            value: &'static AtomicU32,
        }

        pub struct Add(u32);
        impl Actor for MyActor {
            type Message = Add;

            async fn on_mount<'m, M>(&'m mut self, _: DynamicAddress<Add>, mut inbox: M) -> !
            where
                M: Inbox<Add>,
            {
                loop {
                    let message = inbox.next().await;
                    self.value.fetch_add(message.0, Ordering::SeqCst);
                }
            }
        }

        #[embassy_executor::main]
        async fn main(_s: Spawner) {
            static ACTOR: ActorContext<MyActor> = ActorContext::new();

            let a_addr = ACTOR.dyn_address();
            let _ = a_addr.try_notify(Add(10));
            ACTOR
                .mount(MyActor {
                    value: &INITIALIZED,
                })
                .await;
        }

        std::thread::spawn(move || {
            main();
        });

        panic_after(Duration::from_secs(10), move || {
            while INITIALIZED.load(Ordering::SeqCst) != 10 {
                std::thread::sleep(Duration::from_secs(1))
            }
        })
    }

    fn panic_after<T, F>(d: Duration, f: F) -> T
    where
        T: Send + 'static,
        F: FnOnce() -> T,
        F: Send + 'static,
    {
        let (done_tx, done_rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let val = f();
            done_tx.send(()).expect("Unable to send completion signal");
            val
        });

        match done_rx.recv_timeout(d) {
            Ok(_) => handle.join().expect("Thread panicked"),
            Err(_) => panic!("Thread took too long"),
        }
    }
}
