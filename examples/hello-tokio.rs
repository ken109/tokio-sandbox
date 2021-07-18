use std::thread;
use std::time::Duration;

async fn say_world() {
    thread::sleep(Duration::from_secs(4));
    println!("world");
}

#[tokio::main]
async fn main() {
    let sw = tokio::spawn(async { say_world().await; });

    thread::sleep(Duration::from_secs(2));
    println!("hello");

    sw.await.unwrap();
}
