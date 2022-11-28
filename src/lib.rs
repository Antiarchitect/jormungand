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
        println!("SendFut polled!");
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
        println!("RecvFut polled!");
        let mut locked = self.receiver.0.lock();
        let maybe_item = locked.queue.dequeue();
        match maybe_item {
            Some(item) => Poll::Ready(item),
            None => {
                // cx.waker().clone().wake();
                locked.recv_waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

pub fn circular<T: Clone>(cap: usize) -> (Sender<T>, Receiver<T>) {
    let shared = AsyncSafeShared(Arc::new(Mutex::new(Shared {
        queue: RingBuffer::with_capacity(cap),
        recv_waker: None,
    })));
    (Sender(shared.clone()), Receiver(shared))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[test]
    fn tail_rewrite_on_overflow() {
        tokio_test::block_on(async move {
            let (tx, rx) = circular::<u32>(64);

            println!("Started");

            let send_handle = tokio_test::task::spawn(async move {
                for i in 0..65 {
                    tx.send(i as u32).await;
                    println!("Sent {}", i);
                }
            });

            let _send_out = send_handle.await;

            let recv_handle = tokio_test::task::spawn(async move { rx.recv().await });
            assert_eq!(recv_handle.await, 1);
        })
    }

    #[test]
    fn polling_once() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let (tx, rx) = circular::<u32>(64);

            println!("Started");

            let mut recv_timer = tokio::time::interval(std::time::Duration::from_millis(200));
            let handle = tokio::spawn(async move {
                for _i in 0..5 {
                    recv_timer.tick().await;
                    println!("Received {}", rx.recv().await);
                }
            });

            let mut timer = tokio::time::interval(std::time::Duration::from_millis(100));
            let _ = tokio::spawn(async move {
                for i in 0..5 {
                    timer.tick().await;
                    tx.send(i as u32).await;
                    println!("Sent {}", i);
                }
            })
            .await;

            let _ = handle.await;
        })
    }
}
