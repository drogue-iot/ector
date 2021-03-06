use crate::{Actor, ActorSpawner, Address, Inbox};
use atomic_polyfill::{AtomicBool, Ordering};
use core::cell::RefCell;
use core::future::Future;
use core::pin::Pin;
use embassy::channel::signal::Signal;
use embassy::executor::{raw, raw::TaskStorage as Task, SpawnError, Spawner};
use embassy::util::Forever;
use embedded_hal::digital::v2::InputPin;
use embedded_hal_async::digital::Wait;
use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::vec::Vec;

#[derive(Clone, Copy, Default)]
pub struct TestSpawner;

impl ActorSpawner for TestSpawner {
    fn spawn<F: Future<Output = ()> + 'static>(
        &self,
        _: &'static Task<F>,
        _: F,
    ) -> Result<(), SpawnError> {
        Ok(())
    }
}

/// A test context that can execute test for a given device
pub struct TestContext<D: 'static> {
    runner: &'static TestRunner,
    device: &'static Forever<D>,
}

impl<D> TestContext<D> {
    pub fn new(runner: &'static TestRunner, device: &'static Forever<D>) -> Self {
        Self { runner, device }
    }

    /// Configure context with a device
    pub fn configure(&mut self, device: D) -> &'static D {
        self.device.put(device)
    }

    /// Create a test pin that can be used in tests
    pub fn pin(&mut self, initial: bool) -> TestPin {
        self.runner.pin(initial)
    }

    /// Create a signal that can be used in tests
    pub fn signal(&mut self) -> &'static TestSignal {
        self.runner.signal()
    }
}

impl<D> Drop for TestContext<D> {
    fn drop(&mut self) {
        self.runner.done()
    }
}

/// A test message with an id that can be passed around to verify the system
#[derive(Copy, Clone, Debug)]
pub struct TestMessage(pub u32);

/// A dummy actor that does nothing
#[derive(Default)]
pub struct DummyActor {}

impl DummyActor {
    pub fn new() -> Self {
        Self {}
    }
}

impl Actor for DummyActor {
    type Message<'m> = TestMessage;
    type OnMountFuture<'m, M> = impl Future<Output = ()> + 'm where M: 'm + Inbox<TestMessage>;
    fn on_mount<'m, M>(
        &'m mut self,
        _: Address<TestMessage>,
        mut inbox: M,
    ) -> Self::OnMountFuture<'m, M>
    where
        M: Inbox<TestMessage> + 'm,
    {
        async move {
            loop {
                inbox.next().await;
            }
        }
    }
}

/// A test handler that carries a signal that is set on `on_message`
pub struct TestHandler {
    on_message: &'static TestSignal,
}

impl TestHandler {
    pub fn new(signal: &'static TestSignal) -> Self {
        Self { on_message: signal }
    }
}

impl Actor for TestHandler {
    type Message<'m> = TestMessage;
    type OnMountFuture<'m, M> = impl Future<Output = ()> + 'm
    where
        M: 'm + Inbox<TestMessage>;
    fn on_mount<'m, M>(
        &'m mut self,
        _: Address<TestMessage>,
        mut inbox: M,
    ) -> Self::OnMountFuture<'m, M>
    where
        M: Inbox<TestMessage> + 'm,
    {
        async move {
            loop {
                let message = inbox.next().await;
                self.on_message.signal(message);
            }
        }
    }
}

/// A Pin that implements some embassy and embedded_hal traits that can be used to drive device changes.
pub struct TestPin {
    inner: &'static InnerPin,
}

struct InnerPin {
    value: AtomicBool,
    signal: Signal<()>,
}

impl Copy for TestPin {}
impl Clone for TestPin {
    fn clone(&self) -> Self {
        Self { inner: self.inner }
    }
}

impl TestPin {
    pub fn set_high(&self) {
        self.inner.set_value(true)
    }

    pub fn set_low(&self) {
        self.inner.set_value(false)
    }
}

impl InnerPin {
    pub fn new(initial: bool) -> Self {
        Self {
            value: AtomicBool::new(initial),
            signal: Signal::new(),
        }
    }

    fn set_value(&self, value: bool) {
        self.signal.reset();
        self.value.store(value, Ordering::SeqCst);
        self.signal.signal(());
    }

    fn get_value(&self) -> bool {
        self.value.load(Ordering::SeqCst)
    }
}

use core::convert::Infallible;
impl embedded_hal_1::digital::ErrorType for TestPin {
    type Error = Infallible;
}

impl Wait for TestPin {
    type WaitForHighFuture<'m> = impl Future<Output = Result<(), Infallible>> + 'm where Self: 'm;
    fn wait_for_high(&mut self) -> Self::WaitForHighFuture<'_> {
        async move {
            self.inner.signal.wait().await;
            Ok(())
        }
    }

    type WaitForLowFuture<'m> = impl Future<Output = Result<(), Infallible>> + 'm where Self: 'm;
    fn wait_for_low(&mut self) -> Self::WaitForLowFuture<'_> {
        async move {
            self.inner.signal.wait().await;
            Ok(())
        }
    }

    type WaitForRisingEdgeFuture<'m> = impl Future<Output = Result<(), Infallible>> + 'm where Self: 'm;
    fn wait_for_rising_edge(&mut self) -> Self::WaitForRisingEdgeFuture<'_> {
        async move {
            self.inner.signal.wait().await;
            Ok(())
        }
    }
    type WaitForFallingEdgeFuture<'m> = impl Future<Output = Result<(), Infallible>> + 'm where Self: 'm;
    fn wait_for_falling_edge(&mut self) -> Self::WaitForFallingEdgeFuture<'_> {
        async move {
            self.inner.signal.wait().await;
            Ok(())
        }
    }

    type WaitForAnyEdgeFuture<'m> = impl Future<Output = Result<(), Infallible>> + 'm where Self: 'm;
    fn wait_for_any_edge(&mut self) -> Self::WaitForAnyEdgeFuture<'_> {
        async move {
            self.inner.signal.wait().await;
            Ok(())
        }
    }
}

impl InputPin for TestPin {
    type Error = ();
    fn is_high(&self) -> Result<bool, ()> {
        Ok(self.inner.get_value())
    }
    fn is_low(&self) -> Result<bool, ()> {
        Ok(!self.inner.get_value())
    }
}

/// A generic signal construct that can be used across actor and test states.
pub struct TestSignal {
    signal: Signal<()>,
    value: RefCell<Option<TestMessage>>,
}

impl Default for TestSignal {
    fn default() -> Self {
        Self {
            signal: Signal::new(),
            value: RefCell::new(None),
        }
    }
}

impl TestSignal {
    pub fn signal(&self, value: TestMessage) {
        self.value.borrow_mut().replace(value);
        self.signal.signal(())
    }

    pub fn message(&self) -> Option<TestMessage> {
        *self.value.borrow()
    }

    pub async fn wait_signaled(&self) {
        self.signal.wait().await
    }
}

/// A test context that can execute test for a given device
pub struct TestRunner {
    inner: UnsafeCell<raw::Executor>,
    not_send: PhantomData<*mut ()>,
    signaler: &'static Signaler,
    pins: UnsafeCell<Vec<InnerPin>>,
    signals: UnsafeCell<Vec<TestSignal>>,
    done: AtomicBool,
}

impl Default for TestRunner {
    fn default() -> Self {
        let signaler = &*Box::leak(Box::new(Signaler::new()));
        Self {
            inner: UnsafeCell::new(raw::Executor::new(
                |p| unsafe {
                    let s = &*(p as *const () as *const Signaler);
                    s.signal()
                },
                signaler as *const _ as _,
            )),
            not_send: PhantomData,
            signaler,
            pins: UnsafeCell::new(Vec::new()),
            signals: UnsafeCell::new(Vec::new()),
            done: AtomicBool::new(false),
        }
    }
}

impl TestRunner {
    pub fn initialize(&'static self, init: impl FnOnce(Spawner)) {
        init(unsafe { (&*self.inner.get()).spawner() });
    }

    pub fn run_until_idle(&'static self) {
        self.signaler.prepare();
        while self.signaler.should_run() {
            unsafe { (&*self.inner.get()).poll() };
        }
    }

    /// Create a test pin that can be used in tests
    pub fn pin(&'static self, initial: bool) -> TestPin {
        let pins = unsafe { &mut *self.pins.get() };
        pins.push(InnerPin::new(initial));
        TestPin {
            inner: &pins[pins.len() - 1],
        }
    }

    /// Create a signal that can be used in tests
    pub fn signal(&'static self) -> &'static TestSignal {
        let signals = unsafe { &mut *self.signals.get() };
        signals.push(TestSignal::default());
        &signals[signals.len() - 1]
    }

    pub fn done(&'static self) {
        self.done.store(true, Ordering::SeqCst);
    }

    pub fn is_done(&'static self) -> bool {
        self.done.load(Ordering::SeqCst)
    }
}

struct Signaler {
    run: AtomicBool,
}

impl Signaler {
    fn new() -> Self {
        Self {
            run: AtomicBool::new(false),
        }
    }

    fn prepare(&self) {
        self.run.store(true, Ordering::SeqCst);
    }

    fn should_run(&self) -> bool {
        self.run.swap(false, Ordering::SeqCst)
    }

    fn signal(&self) {
        self.run.store(true, Ordering::SeqCst);
    }
}

static mut ALARM_AT: u64 = u64::MAX;
static mut NEXT_ALARM_ID: u8 = 0;

// Perform a process step for an Actor, processing a single message
pub fn step_actor(actor_fut: &mut impl Future<Output = ()>) {
    let waker = futures::task::noop_waker_ref();
    let mut cx = std::task::Context::from_waker(waker);
    let _ = unsafe { Pin::new_unchecked(&mut *actor_fut) }.poll(&mut cx);
}
