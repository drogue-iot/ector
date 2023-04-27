use embassy_executor::SendSpawner;
use embassy_sync::blocking_mutex::raw::RawMutex;

use {core::future::Future, static_cell::StaticCell};

use {
    embassy_executor::{raw::TaskStorage as Task, SpawnError, Spawner},
    embassy_sync::{
        blocking_mutex::raw::NoopRawMutex,
        channel::{Channel, DynamicSender, Receiver, TrySendError},
    },
};

/// Trait that each actor must implement. An actor defines a message type
/// that it acts on, and an implementation of `on_mount` which is invoked
/// when the actor is started.
///
/// At run time, an Actor is held within an ActorContext, which contains the
/// embassy task and the message queues. The size of the message queue is configured
/// per ActorContext.
pub trait Actor: Sized {
    /// The message type that this actor expects to receive from its inbox.
    type Message<'m>
    where
        Self: 'm;

    /// The future type returned in `on_mount`, usually derived from an `async move` block
    /// in the implementation using `impl Trait`.
    type OnMountFuture<'m, M>: Future<Output = ()>
    where
        Self: 'm,
        M: Inbox<Self::Message<'m>> + 'm;

    /// Called when an actor is mounted (activated). The actor will be provided with the
    /// address to itself, and an inbox used to receive incoming messages.
    fn on_mount<'m, M>(
        &'m mut self,
        _: Address<Self::Message<'m>>,
        _: M,
    ) -> Self::OnMountFuture<'m, M>
    where
        M: Inbox<Self::Message<'m>> + 'm;
}

pub trait Inbox<M> {
    /// Retrieve the next message in the inbox. A default value to use as a response must be
    /// provided to ensure a response is always given.
    ///
    /// This method returns None if the channel is closed.
    #[must_use = "Must set response for message"]
    async fn next(&mut self) -> M;
}

impl<'ch, M, MUT, const QUEUE_SIZE: usize> Inbox<M> for Receiver<'ch, MUT, M, QUEUE_SIZE>
where
    M: 'ch,
    MUT: RawMutex,
{
    async fn next(&mut self) -> M {
        self.recv().await
    }
}

/// A handle to another actor for dispatching messages.
///
/// Individual actor implementations may augment the `Address` object
/// when appropriate bounds are met to provide method-like invocations.
pub struct Address<M>
where
    M: 'static,
{
    state: DynamicSender<'static, M>,
}

impl<M> Address<M> {
    fn new(state: DynamicSender<'static, M>) -> Self {
        Self { state }
    }
}

impl<M> Address<M> {
    /// Attempt to send a message to the actor behind this address. If an error
    /// occurs when enqueueing the message on the destination actor, the message is returned.
    pub fn try_notify(&self, message: M) -> Result<(), M> {
        self.state.try_send(message).map_err(|e| match e {
            TrySendError::Full(m) => m,
        })
    }

    // Attempt to deliver a message until successful.
    pub async fn notify(&self, message: M) {
        self.state.send(message).await
    }
}

impl<M, R, MUT> Address<Request<M, R, MUT>>
where
    MUT: RawMutex,
{
    pub async fn request(&self, message: M) -> R {
        let reply_to: Channel<MUT, R, 1> = Channel::new();
        // We guarantee that channel lives until we've been notified on it, at which
        // point its out of reach for the replier.
        let message = Request::new(message, unsafe { core::mem::transmute(&reply_to) });
        self.notify(message).await;
        reply_to.recv().await
    }
}

impl<M> Clone for Address<M> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

type ReplyTo<T, MUT> = Channel<MUT, T, 1>;

pub struct Request<M, R, MUT = NoopRawMutex>
where
    R: 'static,
    MUT: RawMutex + 'static,
{
    message: Option<M>,
    reply_to: &'static ReplyTo<R, MUT>,
}

impl<M, R, MUT> Request<M, R, MUT>
where
    MUT: RawMutex,
{
    fn new(message: M, reply_to: &'static ReplyTo<R, MUT>) -> Self {
        Self {
            message: Some(message),
            reply_to,
        }
    }

    pub async fn process<F: FnOnce(M) -> R>(mut self, f: F) {
        let reply = f(self.message.take().unwrap());
        self.reply_to.send(reply).await;
    }

    pub async fn reply(self, value: R) {
        self.reply_to.send(value).await
    }
}

impl<M, R, MUT> AsRef<M> for Request<M, R, MUT>
where
    MUT: RawMutex,
{
    fn as_ref(&self) -> &M {
        self.message.as_ref().unwrap()
    }
}

impl<M, R, MUT> AsMut<M> for Request<M, R, MUT>
where
    MUT: RawMutex,
{
    fn as_mut(&mut self) -> &mut M {
        self.message.as_mut().unwrap()
    }
}

pub trait ActorSpawner: Clone + Copy {
    fn spawn<F: Future<Output = ()> + 'static>(
        &self,
        task: &'static Task<F>,
        future: F,
    ) -> Result<(), SpawnError>;
}

impl ActorSpawner for Spawner {
    fn spawn<F: Future<Output = ()> + 'static>(
        &self,
        task: &'static Task<F>,
        future: F,
    ) -> Result<(), SpawnError> {
        Spawner::spawn(self, Task::spawn(task, move || future))
    }
}

pub trait SendActorSpawner: Clone + Copy {
    fn spawn<F: Future<Output = ()> + 'static + Send>(
        &self,
        task: &'static Task<F>,
        future: F,
    ) -> Result<(), SpawnError>;
}

impl SendActorSpawner for SendSpawner {
    fn spawn<F: Future<Output = ()> + 'static + Send>(
        &self,
        task: &'static Task<F>,
        future: F,
    ) -> Result<(), SpawnError> {
        SendSpawner::spawn(self, Task::spawn(task, move || future))
    }
}

impl<T> SendActorSpawner for T
where
    T: ActorSpawner,
{
    fn spawn<F: Future<Output = ()> + 'static + Send>(
        &self,
        task: &'static Task<F>,
        future: F,
    ) -> Result<(), SpawnError> {
        Self::spawn(self, task, future)
    }
}

/// A context for an actor, providing signal and message queue. The QUEUE_SIZE parameter
/// is a const generic parameter, and controls how many messages an Actor can handle.
pub struct ActorContext<A, MUT = NoopRawMutex, const QUEUE_SIZE: usize = 1>
where
    A: Actor + 'static,
    MUT: RawMutex + 'static,
{
    task: Task<A::OnMountFuture<'static, Receiver<'static, MUT, A::Message<'static>, QUEUE_SIZE>>>,
    actor: StaticCell<A>,
    channel: Channel<MUT, A::Message<'static>, QUEUE_SIZE>,
}

unsafe impl<A, MUT, const QUEUE_SIZE: usize> Sync for ActorContext<A, MUT, QUEUE_SIZE>
where
    A: Actor,
    MUT: RawMutex,
{
}

impl<A, MUT, const QUEUE_SIZE: usize> Default for ActorContext<A, MUT, QUEUE_SIZE>
where
    A: Actor,
    MUT: RawMutex,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<A, MUT, const QUEUE_SIZE: usize> ActorContext<A, MUT, QUEUE_SIZE>
where
    A: Actor,
    MUT: RawMutex,
{
    pub const fn new() -> Self {
        Self {
            task: Task::new(),
            actor: StaticCell::new(),
            channel: Channel::new(),
        }
    }

    /// Mount the underlying actor and initialize the channel.
    pub fn mount<S: ActorSpawner>(
        &'static self,
        spawner: S,
        actor: A,
    ) -> Address<A::Message<'static>> {
        let (address, future) = self.initialize(actor);
        let task = &self.task;
        // TODO: Map to error?
        spawner.spawn(task, future).unwrap();
        address
    }

    /// Mount the underlying actor and initialize the channel.
    pub fn mount_send<'a, S: SendActorSpawner>(
        &'static self,
        spawner: S,
        actor: A,
    ) -> Address<A::Message<'static>>
    where
        A::OnMountFuture<'static, Receiver<'static, MUT, A::Message<'static>, QUEUE_SIZE>>: Send,
    {
        let (address, future) = self.initialize(actor);
        let task = &self.task;
        // TODO: Map to error?
        spawner.spawn(task, future).unwrap();
        address
    }

    pub fn address(&'static self) -> Address<A::Message<'static>> {
        Address::new(self.channel.sender().into())
    }

    #[allow(clippy::type_complexity)]
    pub(crate) fn initialize(
        &'static self,
        actor: A,
    ) -> (
        Address<A::Message<'static>>,
        A::OnMountFuture<'static, Receiver<'static, MUT, A::Message<'static>, QUEUE_SIZE>>,
    ) {
        let actor = self.actor.init(actor);
        let sender = self.channel.sender();
        let address = Address::new(sender.into());
        let future = actor.on_mount(address.clone(), self.channel.receiver());
        (address, future)
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::testutil::*, core::pin::Pin};
    //use futures::pin_mut;

    #[test]
    fn test_sync_notifications() {
        static ACTOR: ActorContext<DummyActor, NoopRawMutex, 1> = ActorContext::new();

        let (address, mut actor_fut) = ACTOR.initialize(DummyActor::new());

        let result_1 = address.try_notify(TestMessage(0));
        let result_2 = address.try_notify(TestMessage(1));

        assert!(result_1.is_ok());
        assert!(result_2.is_err());

        step_actor(&mut actor_fut);
        let result_2 = address.try_notify(TestMessage(1));
        assert!(result_2.is_ok());
    }

    /*
    #[test]
    fn test_async_notifications() {
        static ACTOR: ActorContext<DummyActor, 1> = ActorContext::new();
        let (address, mut actor_fut) = ACTOR.initialize(DummyActor::new());

        let fut_1 = address.notify(TestMessage(0));
        let _ = address.notify(TestMessage(1));

        let waker = futures::task::noop_waker_ref();
        let mut cx = std::task::Context::from_waker(waker);

        pin_mut!(fut_1);

        while Pin::new(&mut fut_1).poll(&mut cx).is_pending() {
            step_actor(&mut actor_fut);
        }

        let fut_2 = address.notify(TestMessage(1));
        pin_mut!(fut_2);

        while Pin::new(&mut fut_2).poll(&mut cx).is_pending() {
            step_actor(&mut actor_fut);
        }
    }
    */

    // Perform a process step for an Actor, processing a single message
    fn step_actor(actor_fut: &mut impl Future<Output = ()>) {
        let waker = futures::task::noop_waker_ref();
        let mut cx = std::task::Context::from_waker(waker);
        let _ = unsafe { Pin::new_unchecked(&mut *actor_fut) }.poll(&mut cx);
    }
}
