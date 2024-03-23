use futures::stream;
use futures::stream::StreamExt;
use std::time::Duration;
use tokio::pin;
use tokio::time::sleep;

async fn async_work(x: i32) -> i32 {
    sleep(Duration::from_millis(100)).await;

    x
}

async fn async_predicate(x: i32) -> Option<i32> {
    sleep(Duration::from_millis(100)).await;
    (x % 2 == 0).then_some(x)
}

async fn buffered_example() {
    let mut stream = stream::iter(0..10).map(async_work).buffered(3);

    while let Some(next) = stream.next().await {
        println!("next: {}", next);
    }
}

async fn unordered_example() {
    let mut stream = stream::iter(0..10).map(async_work).buffer_unordered(3);

    while let Some(next) = stream.next().await {
        println!("next: {}", next);
    }
}

async fn buffered_filter_example() {
    let stream = stream::iter(0..10)
        .map(async_work)
        .buffered(3)
        .filter_map(async_predicate);

    pin!(stream);

    while let Some(next) = stream.next().await {
        println!("next: {}", next);
    }
}

async fn concurrent_filter_example() {
    let stream = stream::iter(0..10)
        .map(async_predicate)
        .buffered(3)
        .filter_map(futures::future::ready);

    pin!(stream);

    while let Some(next) = stream.next().await {
        println!("next: {}", next);
    }
}

#[tokio::main]
async fn main() {
    // buffered_example().await;
    // unordered_example().await;
    // buffered_filter_example().await;

    concurrent_filter_example().await;
}
