use embassy_sync::{blocking_mutex::raw::RawMutex, channel::Sender};

use static_cell::StaticCell;

use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    channel::{Channel, DynamicSender, Receiver, TrySendError},
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
    type Message;

    /// Called when an actor is mounted (activated). The actor will be provided with the
    /// address to itself, and an inbox used to receive incoming messages.
    async fn on_mount<M>(&mut self, _: DynamicAddress<Self::Message>, _: M) -> !
    where
        M: Inbox<Self::Message>;
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

/// A trait send message to another actor.
///
/// Individual actor implementations may augment the `ActorAddress` object
/// when appropriate bounds are met to provide method-like invocations.
pub trait ActorAddress<M> {
    /// Attempt to send a message to the actor behind this address. If an error
    /// occurs when enqueueing the message on the destination actor, the message is returned.
    fn try_notify(&self, message: M) -> Result<(), M>;

    /// Attempt to deliver a message until successful.
    async fn notify(&self, message: M);
}

pub trait ActorRequest<M, R> {
    /// Attempts to send a message and wait for the response
    async fn request(&self, message: M) -> R;
}

impl<'a, M> ActorAddress<M> for DynamicSender<'a, M> {
    fn try_notify(&self, message: M) -> Result<(), M> {
        self.try_send(message).map_err(|e| match e {
            TrySendError::Full(m) => m,
        })
    }

    async fn notify(&self, message: M) {
        self.send(message).await
    }
}

impl<'a, M, R> ActorRequest<M, R> for DynamicSender<'a, Request<M, R>> {
    async fn request(&self, message: M) -> R {
        let channel: Channel<NoopRawMutex, R, 1> = Channel::new();
        let sender: DynamicSender<'_, R> = channel.sender().into();

        // We guarantee that channel lives until we've been notified on it, at which
        // point its out of reach for the replier.
        let reply_to = unsafe { core::mem::transmute(&sender) };
        let message = Request::new(message, reply_to);
        self.notify(message).await;
        channel.recv().await
    }
}

impl<'a, M, MUT, const N: usize> ActorAddress<M> for Sender<'a, MUT, M, N>
where
    MUT: RawMutex,
{
    fn try_notify(&self, message: M) -> Result<(), M> {
        self.try_send(message).map_err(|e| match e {
            TrySendError::Full(m) => m,
        })
    }

    async fn notify(&self, message: M) {
        self.send(message).await
    }
}

impl<'a, M, R, MUT, const N: usize> ActorRequest<M, R> for Sender<'a, MUT, Request<M, R>, N>
where
    M: 'static,
    MUT: RawMutex + 'static,
{
    async fn request(&self, message: M) -> R {
        let channel: Channel<MUT, R, 1> = Channel::new();
        let sender: DynamicSender<'_, R> = channel.sender().into();

        // We guarantee that channel lives until we've been notified on it, at which
        // point its out of reach for the replier.
        let reply_to = unsafe { core::mem::transmute(&sender) };
        let message = Request::new(message, reply_to);
        self.notify(message).await;
        channel.recv().await
    }
}

/// A handle to another actor for dispatching messages.
///
/// Individual actor implementations may augment the `Address` object
/// when appropriate bounds are met to provide method-like invocations.
pub type DynamicAddress<M> = DynamicSender<'static, M>;

/// A handle to another actor for dispatching messages.
///
/// Individual actor implementations may augment the `Address` object
/// when appropriate bounds are met to provide method-like invocations.
pub type Address<M, MUT, const N: usize = 1> = Sender<'static, MUT, M, N>;

pub struct Request<M, R>
where
    R: 'static,
{
    message: Option<M>,
    reply_to: &'static DynamicSender<'static, R>,
}

/// Safety: Only the [Address] or [DynamicAddress] build requests, meaning that they will use the approriate mutex
/// even if it uses [DynamicSender] internally
/// For [DynamicAddress] a [NoopRawMutex] since it's already not send
/// For [Address], the matching mutex is used
unsafe impl<M, R> Send for Request<M, R> {}

impl<M, R> Request<M, R> {
    fn new(message: M, reply_to: &'static DynamicSender<'static, R>) -> Self {
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

impl<M, R> AsRef<M> for Request<M, R> {
    fn as_ref(&self) -> &M {
        self.message.as_ref().unwrap()
    }
}

impl<M, R> AsMut<M> for Request<M, R> {
    fn as_mut(&mut self) -> &mut M {
        self.message.as_mut().unwrap()
    }
}

/// A context for an actor, providing signal and message queue. The QUEUE_SIZE parameter
/// is a const generic parameter, and controls how many messages an Actor can handle.
pub struct ActorContext<A, MUT = NoopRawMutex, const QUEUE_SIZE: usize = 1>
where
    A: Actor + 'static,
    MUT: RawMutex + 'static,
{
    actor: StaticCell<A>,
    channel: Channel<MUT, A::Message, QUEUE_SIZE>,
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
            actor: StaticCell::new(),
            channel: Channel::new(),
        }
    }

    /// Mount the underlying actor and initialize the channel.
    pub async fn mount(&'static self, actor: A) -> ! {
        let actor = self.actor.init(actor);
        let address = self.channel.sender().into();
        let receiver = self.channel.receiver();
        actor.on_mount(address, receiver).await
    }

    pub fn dyn_address(&'static self) -> DynamicAddress<A::Message> {
        self.channel.sender().into()
    }

    pub fn address(&'static self) -> Address<A::Message, MUT, QUEUE_SIZE> {
        self.channel.sender()
    }
}
