use crate::{Actor, Address, Inbox};
use atomic_polyfill::{AtomicBool, Ordering};
use core::cell::RefCell;
use core::future::Future;
use core::pin::Pin;
use embassy_executor::{raw, Spawner};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use static_cell::StaticCell;
use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::vec::Vec;

type CS = CriticalSectionRawMutex;

/// A test context that can execute test for a given device
pub struct TestContext<D: 'static> {
    runner: &'static TestRunner,
    device: &'static StaticCell<D>,
}

impl<D> TestContext<D> {
    pub fn new(runner: &'static TestRunner, device: &'static StaticCell<D>) -> Self {
        Self { runner, device }
    }

    /// Configure context with a device
    pub fn configure(&mut self, device: D) -> &'static D {
        self.device.init(device)
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
    type Message = TestMessage;

    async fn on_mount<M>(&mut self, _: Address<TestMessage>, mut inbox: M) -> !
    where
        M: Inbox<TestMessage>,
    {
        loop {
            inbox.next().await;
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
    type Message = TestMessage;

    async fn on_mount<M>(&mut self, _: Address<TestMessage>, mut inbox: M) -> !
    where
        M: Inbox<TestMessage>,
    {
        loop {
            let message = inbox.next().await;
            self.on_message.signal(message);
        }
    }
}

/// A Pin that implements some embassy and embedded_hal traits that can be used to drive device changes.
pub struct TestPin {
    inner: &'static InnerPin,
}

struct InnerPin {
    value: AtomicBool,
    signal: Signal<CS, ()>,
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

/// A generic signal construct that can be used across actor and test states.
pub struct TestSignal {
    signal: Signal<CS, ()>,
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
        init(unsafe { (*self.inner.get()).spawner() });
    }

    pub fn run_until_idle(&'static self) {
        self.signaler.prepare();
        while self.signaler.should_run() {
            unsafe { (*self.inner.get()).poll() };
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

// Perform a process step for an Actor, processing a single message
pub fn step_actor<R>(actor_fut: &mut impl Future<Output = R>) {
    let waker = futures::task::noop_waker_ref();
    let mut cx = std::task::Context::from_waker(waker);
    let _ = unsafe { Pin::new_unchecked(&mut *actor_fut) }.poll(&mut cx);
}
