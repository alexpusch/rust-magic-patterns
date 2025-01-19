use std::time::Duration;

use futures::{stream::FuturesUnordered, StreamExt};
use tokio::sync::mpsc;

struct Image {
    url: String,
    data: Vec<u8>,
}

async fn download_image(url: String) -> Image {
    println!("downloading {url}");
    tokio::time::sleep(Duration::from_millis(5)).await;

    Image { url, data: vec![0] }
}

async fn process_image(image: Image) -> Image {
    println!("processing image {}", image.url);
    tokio::time::sleep(Duration::from_millis(20)).await;

    Image {
        url: image.url,
        data: vec![1],
    }
}

async fn save_image(image: Image) {
    println!("saving image {}", image.url);
    tokio::time::sleep(Duration::from_millis(5)).await;
}

async fn async_pipeline_example() {
    let urls = (0..32).map(|i| format!("https://example.com/image/{}", i));

    let (url_sender, mut url_receiver) = mpsc::channel(100);
    let (image_sender, mut image_receiver) = mpsc::channel(100);
    let (processed_sender, mut processed_receiver) = mpsc::channel(100);
    let (output_sender, mut output_receiver) = mpsc::channel(100);

    let h1 = tokio::spawn(async move {
        while let Some(url) = url_receiver.recv().await {
            let image = download_image(url).await;
            if let Err(err) = image_sender.send(image).await {
                println!("failed to send output: {}", err);
                break;
            }
        }
    });

    let h2 = tokio::spawn(async move {
        // process concurrently up to 4 images
        let mut futures = FuturesUnordered::new();

        loop {
            let in_progress_len = futures.len();

            tokio::select! {
                biased;

                Some(image) = image_receiver.recv(), if in_progress_len < 4 => {
                    futures.push(process_image(image));
                },
                Some(processed_image) = futures.next(), if in_progress_len > 0 => {
                    if let Err(err) = processed_sender.send(processed_image).await {
                        println!("failed to send output: {}", err);
                        break;
                    }
                },
                else => break
            }
        }
    });

    let h3 = tokio::spawn(async move {
        while let Some(image) = processed_receiver.recv().await {
            let image_url = image.url.clone();
            save_image(image).await;
            if let Err(err) = output_sender.send(image_url).await {
                println!("failed to send output: {}", err);
                break;
            }
        }
    });

    for url in urls {
        url_sender.send(url).await.unwrap();
    }

    // drop sender to make channel finite
    drop(url_sender);

    while let Some(url) = output_receiver.recv().await {
        println!("done with {url}");
    }

    tokio::try_join!(h1, h2, h3).unwrap();
}

#[tokio::main()]
async fn main() {
    async_pipeline_example().await;
}
