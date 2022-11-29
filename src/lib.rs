use std::{
    clone::Clone,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Waker},
};

use parking_lot::Mutex;
use ringbuffer::AllocRingBuffer as RingBuffer;
use ringbuffer::{RingBufferRead, RingBufferWrite};

#[derive(Clone)]
struct AsyncSafeShared<T>(Arc<Mutex<Shared<T>>>);

struct Shared<T> {
    queue: RingBuffer<T>,
    recv_waker: Option<Waker>,
}

#[derive(Clone)]
pub struct Sender<T>(AsyncSafeShared<T>);

impl<T: Clone> Sender<T> {
    pub fn send(&self, item: T) -> SendFut<T> {
        SendFut {
            sender: self.0.clone(),
            item,
        }
    }
}

pub struct SendFut<T> {
    sender: AsyncSafeShared<T>,
    item: T,
}

impl<T: Clone> Future for SendFut<T> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut locked = self.sender.0.lock();
        locked.queue.enqueue(self.item.clone());
        if let Some(waker) = locked.recv_waker.take() {
            waker.wake()
        }
        Poll::Ready(())
    }
}

#[derive(Clone)]
pub struct Receiver<T>(AsyncSafeShared<T>);

impl<T: Clone> Receiver<T> {
    pub fn recv(&self) -> RecvFut<T> {
        RecvFut {
            receiver: self.0.clone(),
        }
    }
}

pub struct RecvFut<T> {
    receiver: AsyncSafeShared<T>,
}

impl<T> Future for RecvFut<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut locked = self.receiver.0.lock();
        let maybe_item = locked.queue.dequeue();
        match maybe_item {
            Some(item) => Poll::Ready(item),
            None => {
                locked.recv_waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

pub fn new<T: Clone>(cap: usize) -> (Sender<T>, Receiver<T>) {
    let shared = AsyncSafeShared(Arc::new(Mutex::new(Shared {
        queue: RingBuffer::with_capacity(cap),
        recv_waker: None,
    })));
    (Sender(shared.clone()), Receiver(shared))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_rewrite_on_overflow() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let (tx, rx) = new::<u32>(64);

            println!("Started");

            tokio::spawn(async move {
                for i in 0..65 {
                    tx.send(i as u32).await;
                    println!("Sent {}", i);
                }
            })
            .await
            .expect("Sender part has failed!");

            assert_eq!(
                tokio::spawn(async move { rx.recv().await })
                    .await
                    .expect("Receiver part has failed!"),
                1
            );
        })
    }

    #[test]
    fn polling_once() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let (tx, rx) = new::<u32>(64);

            println!("Started");

            let mut receiver_timer = tokio::time::interval(std::time::Duration::from_millis(20));
            let receiver_handle = tokio::spawn(async move {
                for _i in 0..5 {
                    receiver_timer.tick().await;
                    println!("Received {}", rx.recv().await);
                }
            });

            let mut sender_timer = tokio::time::interval(std::time::Duration::from_millis(10));
            tokio::spawn(async move {
                for i in 0..5 {
                    sender_timer.tick().await;
                    tx.send(i as u32).await;
                    println!("Sent {}", i);
                }
            })
            .await
            .expect("Sender part has failed!");

            receiver_handle.await.expect("Receiver part has failed!");
        })
    }
}
