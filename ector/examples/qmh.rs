//! Queued Message Handler (QMH) for async actor message handling.
//!
//! This is a simple pattern for handling specific messages in an actor.
//! The actor has private data and a message handler that is called when a message is received.
//! It can also be set to run a sequence of messages on a schedule.

use core::future::pending;
use embassy_time::{Duration, Timer};
use qmh_actor::*;

#[embassy_executor::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    let config = Config {
        starting_count: 0,
        schedule: Duration::from_secs(2),
    };
    let inbox_1 = spawn_actor(spawner, config).expect("ActorQMH failed to start");

    Timer::after_secs(1).await;
    // Send a message to the actor
    inbox_1.send(Message::Act(1)).await;

    // intermediate messages won't abort the schedule
    Timer::after_secs(1).await;
    inbox_1.send(Message::Echo("Hello".to_string())).await;
    Timer::after_secs(1).await;
    inbox_1.send(Message::Echo("Hello".to_string())).await;

    Timer::after_secs(5).await;
    inbox_1.send(Message::Stop).await;
    // the sequencer will not send another message, as the stop message will interrupt the scheduler

    pending().await
}

pub mod qmh_actor {
    use actor_internals::*;
    use core::future::pending;
    use ector::{mutex::NoopRawMutex, *};
    use embassy_executor::SpawnError;
    use embassy_futures::select::{select, Either};
    use embassy_sync::channel::Sender;
    use embassy_time::{Duration, Instant, Timer};

    /// Alias for the actor's inbox
    pub type ActorInbox<M> = Sender<'static, NoopRawMutex, M, 1>;

    /// The actor's message type, communicating the finite states of the actor.
    /// This is made available to other actors to interact with this one.
    pub enum Message {
        /// Act on a message, start a schedule
        Act(usize),
        /// Stop the schedule
        Stop,
        /// Echo the message
        Echo(String),
    }

    /// The actor's configuration, to be shared with other actors to initialize this actor.
    pub struct Config {
        pub starting_count: usize,
        pub schedule: Duration,
    }

    /// Create a new actor with a spawner and a configuration.
    /// This pattern could be made into a macro to simplify the actor creation.
    pub fn spawn_actor(
        spawner: embassy_executor::Spawner,
        config: Config,
    ) -> Result<ActorInbox<Message>, SpawnError> {
        static CONTEXT: ActorContext<ActorQMH> = ActorContext::new();
        let inbox = CONTEXT.address();
        spawner.spawn(actor_task(&CONTEXT, ActorQMH::new(spawner, config, inbox)))?;
        Ok(inbox)
    }

    /// The actor's private data, not to be shared with other actors.
    mod actor_internals {
        use super::*;

        /// A scheduler to run a sequence of messages
        struct Scheduler {
            /// The timer to schedule the next message
            timer: Timer,
            /// The starting time of the schedule
            start: Instant,
        }

        /// The actor's private data, not to be shared with other actors.
        /// This is where the actor's state is stored.
        pub(super) struct ActorQMH {
            /// A timer to schedule the next message
            scheduler: Option<Scheduler>,
            /// Some data to be used in the message handler
            count: usize,
            /// The duration of each iteration of the schedule
            period: Duration,
        }

        impl Actor for ActorQMH {
            type Message = Message;

            /// Actor pattern for either handling new incoming messages or running a scheduled action.
            async fn on_mount<M>(&mut self, _: DynamicAddress<Message>, mut inbox: M) -> !
            where
                M: Inbox<Self::Message>,
            {
                println!("QMH started!");
                loop {
                    let deadline = async {
                        match self.scheduler.as_mut() {
                            Some(Scheduler {
                                timer: next_timer, ..
                            }) => next_timer.await,
                            None => pending().await,
                        }
                    };
                    match select(inbox.next(), deadline).await {
                        Either::First(action) => self.act(action).await,
                        Either::Second(_) => self.next().await,
                    }
                }
            }
        }

        impl ActorQMH {
            /// Create a new actor with a spawner and a configuration.
            pub(super) fn new(
                _spawner: embassy_executor::Spawner,
                config: Config,
                _inbox: ActorInbox<Message>,
            ) -> Self {
                // Opportunity to do any setup before mounting the actor
                // this could include spawning child actors or setting up resources
                // we have access to our own inbox here to send down to child actors.
                Self {
                    scheduler: None,
                    count: config.starting_count,
                    period: config.schedule,
                }
            }
            /// The message handler
            async fn act(&mut self, msg: Message) {
                match msg {
                    Message::Act(n) => {
                        // Set the starting count and begin the schedule
                        self.count += n;
                        self.scheduler = Some(Scheduler {
                            timer: Timer::after(self.period),
                            start: Instant::now(),
                        });
                        println!("Acting: {}", n);
                    }
                    Message::Stop => {
                        // Stop the schedule
                        println!("Stopping");
                        self.scheduler = None;
                    }
                    Message::Echo(s) => {
                        println!("Echoing: {}", s);
                    }
                }
            }
            /// Run the next scheduled action.
            async fn next(&mut self) {
                self.count += 1;
                if let Some(Scheduler { timer, start }) = self.scheduler.as_mut() {
                    println!("Next: {} @ {:?}ms", self.count, start.elapsed().as_millis());
                    *timer = Timer::after(self.period);
                }
            }
        }

        #[embassy_executor::task]
        /// The actor's task, to be spawned by the actor's context.
        pub(super) async fn actor_task(context: &'static ActorContext<ActorQMH>, actor: ActorQMH) {
            context.mount(actor).await;
        }
    }
}
