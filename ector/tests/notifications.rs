#![allow(dead_code)]
#![allow(incomplete_features)]
#![warn(unused_attributes)]

use ector::{
    testutils::{DummyActor, *},
    *,
};

#[test]
fn test_sync_notifications() {
    static ACTOR: ActorContext<DummyActor, mutex::NoopRawMutex, 1> = ActorContext::new();
    let address = ACTOR.dyn_address();
    let mut actor_fut = ACTOR.mount(DummyActor::new());

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
