<p align="center">
    <a href="https://crates.io/crates/jormungand">
         <img src="https://img.shields.io/crates/v/jormungand.svg?style=flat-square" alt="crates.io">
    </a>
    <a href="https://opensource.org/licenses/MIT">
        <img src="https://img.shields.io/badge/License-MIT-blue.svg">
    </a>
</p>

## Jormungand

Jormungand is an async and thread safe circular buffer that rewrites old items on overflow.
Fits well to protect ones server from slow-consuming clients when data loss is acceptable in order to maintain fixed data lag and prevent memory bloat.

## Examples

### Simple

./examples/simple.rs

```rust
use jormungand;
use tokio;

#[tokio::main]
async fn main() {
    let (tx, rx) = jormungand::new::<u32>(64);

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
}
```
