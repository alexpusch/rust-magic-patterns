use std::{
    f32::consts::PI,
    time::{Duration, Instant},
};

use futures::FutureExt;
use pyo3::{ffi::c_str, prelude::*, IntoPyObject, Python};
use serde::Serialize;
use tokio::sync::mpsc;

const N: usize = 500;
const K: f32 = 16. * PI / N as f32;

#[derive(Serialize, Debug, IntoPyObject, Clone)]
struct Sample {
    fut_name: String,
    value: f32,
    start: u128,
    end: u128,
    thread_id: usize,
}

fn sin(i: usize) -> f32 {
    std::thread::sleep(Duration::from_micros(100));
    (K * i as f32).sin()
}

fn sin_heavy(i: usize) -> f32 {
    std::thread::sleep(Duration::from_micros(500));
    (K * i as f32).sin()
}

async fn produce_sin(
    run_start: Instant,
    fut_name: impl ToString,
    tx: mpsc::UnboundedSender<Sample>,
) {
    for i in 1..N {
        let start = run_start.elapsed().as_micros();
        let value = sin(i);
        let end = run_start.elapsed().as_micros();

        let sample = Sample {
            fut_name: fut_name.to_string(),
            value,
            start,
            end,
            thread_id: thread_id::get(),
        };

        tx.send(sample).unwrap();
        tokio::task::yield_now().await;
    }
}

async fn produce_sin_heavy(
    run_start: Instant,
    fut_name: impl ToString,
    tx: mpsc::UnboundedSender<Sample>,
) {
    for i in 1..N {
        let start = run_start.elapsed().as_micros();
        let value = sin_heavy(i);
        let end = run_start.elapsed().as_micros();

        let sample = Sample {
            fut_name: fut_name.to_string(),
            value,
            start,
            end,
            thread_id: thread_id::get(),
        };

        tx.send(sample).unwrap();
        tokio::task::yield_now().await;
    }
}

async fn produce_sin_heavy_blocking(
    run_start: Instant,
    fut_name: impl ToString,
    tx: mpsc::UnboundedSender<Sample>,
) {
    for i in 1..N {
        let start = run_start.elapsed().as_micros();
        let tx = tx.clone();

        let (t_id, value) = tokio::task::spawn_blocking(move || {
            let value = sin_heavy(i);
            let t_id = thread_id::get();

            (t_id, value)
        })
        .await
        .unwrap();

        let end = run_start.elapsed().as_micros();

        let sample = Sample {
            fut_name: fut_name.to_string(),
            value,
            start,
            end,
            thread_id: t_id,
        };

        tx.send(sample).unwrap();

        tokio::task::yield_now().await;
    }
}

fn plot_samples(
    samples: Vec<Sample>,
    include_times: bool,
    output_filename: &str,
) -> Result<(), pyo3::PyErr> {
    let code = c_str!(include_str!("./py/plot.py"));

    Python::with_gil(|py| -> PyResult<()> {
        let module = PyModule::from_code(py, code, c_str!("plot.py"), c_str!("plot"))?;
        let plot_fn = module.getattr("plot")?;

        plot_fn.call1((samples, include_times, output_filename))?;

        Ok(())
    })
}

async fn two_futures() -> Vec<Sample> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut futs = Vec::new();

    let run_start = Instant::now();

    futs.push(produce_sin(run_start, "fut0", tx.clone()).boxed());
    futs.push(produce_sin(run_start, "fut1", tx.clone()).boxed());

    futures::future::join_all(futs).await;
    drop(tx);

    let mut samples = Vec::new();

    while let Some(next) = rx.recv().await {
        samples.push(next);
    }

    // draw_samples(samples, false, "output/two_futures.png").unwrap();
    samples
}

async fn cpu_intensive() -> Vec<Sample> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut futs = Vec::new();

    let run_start = Instant::now();

    futs.push(produce_sin(run_start, "fut0", tx.clone()).boxed());
    futs.push(produce_sin(run_start, "fut1", tx.clone()).boxed());
    futs.push(produce_sin_heavy(run_start, "high cpu", tx.clone()).boxed());

    futures::future::join_all(futs).await;
    drop(tx);

    let mut samples = Vec::new();

    while let Some(next) = rx.recv().await {
        samples.push(next);
    }

    samples
}

async fn spawn_task() -> Vec<Sample> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut futs = Vec::new();

    let run_start = Instant::now();

    futs.push(produce_sin(run_start, "fut0", tx.clone()).boxed());
    futs.push(produce_sin(run_start, "fut1", tx.clone()).boxed());

    futs.push(
        tokio::spawn(produce_sin_heavy(run_start, "spawned", tx.clone()).boxed())
            .map(|_| ())
            .boxed(),
    );

    futures::future::join_all(futs).await;
    drop(tx);

    let mut samples = Vec::new();

    while let Some(next) = rx.recv().await {
        samples.push(next);
    }

    samples
}

async fn many_spawn_task() -> Vec<Sample> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut futs = Vec::new();

    let run_start = Instant::now();

    futs.push(produce_sin(run_start, "fut0", tx.clone()).boxed());

    for i in 1..7 {
        futs.push(
            tokio::spawn(produce_sin_heavy(run_start, format!("spawned{i}"), tx.clone()).boxed())
                .map(|_| ())
                .boxed(),
        );
    }

    futures::future::join_all(futs).await;
    drop(tx);

    let mut samples = Vec::new();

    while let Some(next) = rx.recv().await {
        samples.push(next);
    }

    samples
}

async fn many_spawn_blocking() -> Vec<Sample> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut futs = Vec::new();

    let run_start = Instant::now();

    futs.push(produce_sin(run_start, "fut0", tx.clone()).boxed());

    for i in 1..7 {
        futs.push(
            produce_sin_heavy_blocking(run_start, format!("spawn_\nblocking{i}"), tx.clone())
                .boxed(),
        );
    }

    futures::future::join_all(futs).await;
    drop(tx);

    let mut samples = Vec::new();

    while let Some(next) = rx.recv().await {
        samples.push(next);
    }

    samples
}

fn zoom(samples: Vec<Sample>, ratio: f32) -> Vec<Sample> {
    let min_start = samples.iter().map(|s| s.start).min().unwrap();
    let max_end = samples.iter().map(|s| s.end).max().unwrap();

    let threshold = ((max_end - min_start) as f32 * ratio) as u128;

    samples
        .into_iter()
        .filter(|s| s.start < threshold)
        .collect()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pyo3::prepare_freethreaded_python();

    let two_futures_samples = two_futures().await;
    plot_samples(
        two_futures_samples.clone(),
        false,
        "resources/two_futures.png",
    )?;
    plot_samples(
        two_futures_samples.clone(),
        true,
        "resources/two_futures_with_times.png",
    )?;
    plot_samples(
        zoom(two_futures_samples, 0.05),
        true,
        "resources/two_futures_zoom.png",
    )?;

    let spawn_task_samples = spawn_task().await;
    plot_samples(spawn_task_samples.clone(), true, "resources/spawn_task.png")?;
    plot_samples(
        zoom(spawn_task_samples, 0.05),
        true,
        "resources/spawn_task_zoom.png",
    )?;

    let many_spawn_task_samples = many_spawn_task().await;
    plot_samples(
        many_spawn_task_samples.clone(),
        true,
        "resources/_many_spawn_task.png",
    )?;
    plot_samples(
        zoom(many_spawn_task_samples, 0.05),
        true,
        "resources/_many_spawn_task_zoom.png",
    )?;

    let cpu_intensive_samples = cpu_intensive().await;
    plot_samples(
        cpu_intensive_samples.clone(),
        true,
        "resources/cpu_intensive.png",
    )?;
    plot_samples(
        zoom(cpu_intensive_samples, 0.05),
        true,
        "resources/cpu_intensive_zoom.png",
    )?;

    let many_spawn_blocking_samples = many_spawn_blocking().await;
    plot_samples(
        many_spawn_blocking_samples.clone(),
        true,
        "resources/many_spawn_blocking.png",
    )?;
    plot_samples(
        zoom(many_spawn_blocking_samples, 0.05),
        true,
        "resources/many_spawn_blocking_zoom.png",
    )?;

    Ok(())
}
