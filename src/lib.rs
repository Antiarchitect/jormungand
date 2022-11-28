use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    sync::Arc,
};

use ringbuffer::AllocRingBuffer as RingBuffer;
use ringbuffer::{RingBufferRead, RingBufferWrite};
use parking_lot::Mutex;

type Shared<T> = Arc<Mutex<RingBuffer<T>>>;

pub struct Sender<T> {
    shared: Shared<T>
}

impl<T> Sender<T> {
    pub fn send(&self, item: T) -> SendFut<T> {
        SendFut {
            sender: self.shared.clone(),
            item: item,
        }
    }
}

pub struct SendFut<T> {
    sender: Shared<T>,
    item: T,
}

impl<T: Clone> Future for SendFut<T> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.sender.lock().enqueue(self.item.clone());
        Poll::Ready(())
    }
}

pub struct Receiver<T> {
    shared: Shared<T>
}

impl<T> Receiver<T> {
    pub fn recv(&self) -> RecvFut<T> {
        RecvFut { receiver: self.shared.clone() }
    }
}

pub struct RecvFut<T> {
    receiver: Shared<T>,
}

impl<T: Clone + std::fmt::Debug> Future for RecvFut<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.receiver.lock().dequeue() {
            Some(item) => {
                Poll::Ready(item.clone())
            }
            None => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }
}

pub fn circular<T>(cap: usize) -> (Sender<T>, Receiver<T>) {
    let shared = Arc::new(Mutex::new(RingBuffer::with_capacity(cap)));
    (
        Sender { shared: shared.clone() },
        Receiver { shared },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[test]
    fn tail_rewrite_on_overflow() {
        tokio_test::block_on(
            async move {
                let (tx, rx) = circular::<u32>(64);

                println!("Started");

                let send_handle = tokio_test::task::spawn(async move {
                    for i in 0..65 {
                        tx.send(i as u32).await;
                        println!("Sent {}", i);
                    };
                });

                let _send_out = send_handle.await;

                let recv_handle = tokio_test::task::spawn(async move {
                    rx.recv().await
                });
                assert_eq!(recv_handle.await, 1);
            }

        )
    }
}
