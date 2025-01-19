use std::time::{Duration, Instant};

use futures::{stream, StreamExt};
use plotly::{
    layout::{Axis, AxisType, BarMode},
    Bar, ImageFormat, Layout, Plot,
};

use pumps::Concurrency;
use rand::{rngs::StdRng, Rng, SeedableRng};

async fn work(i: usize, duration: Duration) -> usize {
    tokio::time::sleep(duration / 2).await;
    tokio::time::sleep(duration / 2).await;

    i
}

async fn run_with_stream(
    n: usize,
    timings1: Vec<Duration>,
    timings2: Vec<Duration>,
    timings3: Vec<Duration>,
    concurrency: usize,
) {
    let input = stream::iter(0..n);

    let mut stream = input
        .map(|i| work(i, timings1[i]))
        .buffer_unordered(concurrency)
        .map(|i| work(i, timings2[i]))
        .buffer_unordered(concurrency)
        .map(|i| work(i, timings3[i]))
        .buffer_unordered(concurrency);

    let mut sum = 0usize;
    while let Some(i) = stream.next().await {
        sum += i;
    }

    // make sure we processed all items
    assert_eq!(sum, (n - 1) * n / 2);
}

async fn run_with_pumps(
    n: usize,
    timings1: Vec<Duration>,
    timings2: Vec<Duration>,
    timings3: Vec<Duration>,
    concurrency: usize,
) {
    let input = 0..n;

    let (mut reciver, handler) = pumps::Pipeline::from_iter(input)
        .map(
            move |i| work(i, timings1[i]),
            Concurrency::concurrent_unordered(concurrency),
        )
        .map(
            move |i| work(i, timings2[i]),
            Concurrency::concurrent_unordered(concurrency),
        )
        .map(
            move |i| work(i, timings3[i]),
            Concurrency::concurrent_ordered(concurrency),
        )
        .build();

    let mut sum = 0usize;
    while let Some(i) = reciver.recv().await {
        sum += i;
    }

    // make sure we processed all items
    assert_eq!(sum, (n - 1) * n / 2);

    handler.await.unwrap();
}

async fn run_with_pumps_with_backpressure(
    n: usize,
    timings1: Vec<Duration>,
    timings2: Vec<Duration>,
    timings3: Vec<Duration>,
    concurrency: usize,
) {
    let input = 0..n;

    let (mut reciver, handler) = pumps::Pipeline::from_iter(input)
        .backpressure(128)
        .map(
            move |i| work(i, timings1[i]),
            Concurrency::concurrent_unordered(concurrency),
        )
        .backpressure(128)
        .map(
            move |i| work(i, timings2[i]),
            Concurrency::concurrent_unordered(concurrency),
        )
        .backpressure(128)
        .map(
            move |i| work(i, timings3[i]),
            Concurrency::concurrent_ordered(concurrency),
        )
        .backpressure(128)
        .build();

    let mut sum = 0usize;
    while let Some(i) = reciver.recv().await {
        sum += i;
    }

    // make sure we processed all items
    assert_eq!(sum, (n - 1) * n / 2);

    handler.await.unwrap();
}

fn generate_timings(rng: &mut StdRng, n: usize, durations: (u32, u32)) -> Vec<Duration> {
    (0..n)
        .map(|_i| rng.gen_range(durations.0..durations.1))
        .map(|ms| Duration::from_millis(ms as u64))
        .collect::<Vec<_>>()
}

fn generate_timings_with_slowdowns(
    rng: &mut StdRng,
    n: usize,
    durations: (u32, u32),
) -> Vec<Duration> {
    (0..n)
        .map(|_i| {
            if rng.gen_bool(0.8) {
                rng.gen_range(durations.0..durations.1)
            } else {
                durations.1 * 2
            }
        })
        .map(|ms| Duration::from_millis(ms as u64))
        .collect::<Vec<_>>()
}

async fn bench_by_concurrency(
    n: usize,
    durations: &[(u32, u32)],
    concurrencies: &[usize],
    title: &str,
) {
    println!(
        "Running {title} with {n} items, concurrencies - {concurrencies:?}, durations: {durations:?}",
    );

    let mut x_labels = vec![];
    let mut stream_y_labels = vec![];
    let mut pumps_y_labels = vec![];

    let mut rng: StdRng = SeedableRng::from_seed([100; 32]);

    let timings = generate_timings(&mut rng, n, durations[0]);
    let timings2 = generate_timings(&mut rng, n, durations[1]);
    let timings3 = generate_timings(&mut rng, n, durations[2]);

    for concurrency in concurrencies {
        x_labels.push(*concurrency);

        println!("\tRunning with concurrency = {concurrency}",);

        let start = Instant::now();
        run_with_stream(
            n,
            timings.clone(),
            timings2.clone(),
            timings3.clone(),
            *concurrency,
        )
        .await;
        println!("\t\tstream runtime: {:?}", start.elapsed());
        stream_y_labels.push(start.elapsed().as_millis());

        let start = Instant::now();
        run_with_pumps(
            n,
            timings.clone(),
            timings2.clone(),
            timings3.clone(),
            *concurrency,
        )
        .await;
        println!("\t\tpumps runtime: {:?}", start.elapsed());
        pumps_y_labels.push(start.elapsed().as_millis());
    }

    let layout = Layout::new()
        .bar_mode(BarMode::Group)
        .x_axis(Axis::new().type_(AxisType::Category).title("concurrency"))
        .y_axis(Axis::new().title("milliseconds"))
        .title(title);

    let mut plot = Plot::new();
    plot.set_layout(layout);

    let stream_trace = Bar::new(x_labels.clone(), stream_y_labels).name("stream");
    let pumps_trace = Bar::new(x_labels.clone(), pumps_y_labels).name("pumps");

    plot.add_trace(stream_trace.clone());
    plot.add_trace(pumps_trace);

    let filename = format!("concurrency_{:?}_{:?}", durations, concurrencies);
    plot.write_image(filename, ImageFormat::PNG, 600, 400, 1.0);
}

async fn bench_by_concurrency_with_backpressure(
    n: usize,
    durations: &[(u32, u32)],
    concurrencies: &[usize],
    title: &str,
) {
    println!(
        "Running {title} with {n} items, concurrencies - {concurrencies:?}, durations: {durations:?}",
    );

    let mut x_labels = vec![];
    let mut pumps_bp_y_labels = vec![];
    let mut pumps_y_labels = vec![];

    let mut rng: StdRng = SeedableRng::from_seed([100; 32]);

    let timings = generate_timings_with_slowdowns(&mut rng, n, durations[0]);
    let timings2 = generate_timings_with_slowdowns(&mut rng, n, durations[1]);
    let timings3 = generate_timings_with_slowdowns(&mut rng, n, durations[2]);

    for concurrency in concurrencies {
        x_labels.push(*concurrency);

        println!("\tRunning with concurrency = {concurrency}");

        let start = Instant::now();
        run_with_pumps_with_backpressure(
            n,
            timings.clone(),
            timings2.clone(),
            timings3.clone(),
            *concurrency,
        )
        .await;
        println!("\t\tpumps with backpressure runtime: {:?}", start.elapsed());
        pumps_bp_y_labels.push(start.elapsed().as_millis());

        let start = Instant::now();
        run_with_pumps(
            n,
            timings.clone(),
            timings2.clone(),
            timings3.clone(),
            *concurrency,
        )
        .await;
        println!("\t\tpumps runtime: {:?}", start.elapsed());
        pumps_y_labels.push(start.elapsed().as_millis());
    }

    let layout = Layout::new()
        .bar_mode(BarMode::Group)
        .x_axis(Axis::new().type_(AxisType::Category).title("concurrency"))
        .y_axis(Axis::new().title("milliseconds"))
        .title(title);

    let mut plot = Plot::new();
    plot.set_layout(layout);

    let pumps_bp_trace =
        Bar::new(x_labels.clone(), pumps_bp_y_labels).name("pumps /w backpressure");
    let pumps_trace = Bar::new(x_labels.clone(), pumps_y_labels).name("pumps");

    plot.add_trace(pumps_trace);
    plot.add_trace(pumps_bp_trace.clone());

    let filename = format!("backpressure_{:?}_{:?}.png", durations, concurrencies);
    plot.write_image(filename, ImageFormat::PNG, 600, 400, 1.0);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let n = 1000;

    bench_by_concurrency(
        n,
        &[(5, 10), (5, 10), (5, 10)],
        &[1, 2, 4, 8],
        "Pipeline runtime by concurrency",
    )
    .await;

    bench_by_concurrency(
        n,
        &[(5, 10), (5, 10), (5, 10)],
        &[16, 32, 64, 128, 256],
        "Pipeline runtime by concurrency - high concurrency",
    )
    .await;

    bench_by_concurrency_with_backpressure(
        n,
        &[(5, 10), (5, 10), (5, 10)],
        &[1, 2, 4, 8],
        "Pipeline runtime by concurrency - backpressure",
    )
    .await;

    Ok(())
}
