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
