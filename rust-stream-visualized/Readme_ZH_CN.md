# 可视化探索 Rust 流接口 (Stream API)

在实际应用程序中管理并发相当棘手。开发人员必须处理并发控制、背压、错误处理等问题。幸运的是，`Rust` 为我们提供了 `async/await` 机制，并且在此基础上，还有 [流接口](https://docs.rs/futures/latest/futures/stream/index.html)。

流方法允许我们优雅地定义一个异步操作管道，并提供一个很好的抽象来解决常见的用例。

很遗憾，优雅有时会掩盖复杂性。你能看出一个流管道中有多少操作会并行运行吗？它们的执行顺序又是怎样的？我发现这比看起来要复杂得多，所以我自然而然地编写了一个 `Bevy` 可视化工具来研究这个问题。这项调查揭示了一些完全出乎意料的结果 - 出乎意料到在某些情况下，你可能需要重新考虑使用这个接口。

## 流接口的概述

让我们从流接口的简要概述开始。以下代码定义了一个异步管道，它遍历从 `0` 到 `10` 的整数，限制 `async_work` 方法的并发数量为 3 并执行。然后使用 `async_predicate` 方法对结果进行过滤。这真是太棒了！通过几行代码，我们就创建了一个非平凡的异步控制流。

```rust
async fn async_work(i32) -> i32 {...}
async fn async_predicate(i32) -> Option<i32> {...}

async fn buffered_filter_example() {
    let stream = stream::iter(0..10)
        .map(async_work) // async_work 返回一个 future。这个阶段的输出是一个 futures 流
        .buffered(3) // 轮询 futures 流，并最多同时运行 3 个
        .filter_map(async_predicate); // 使用 async_predicate 函数过滤掉前一阶段的结果

    pin!(stream);

    while let Some(next) = stream.next().await {
        println!("finished working on: {}", next);
    }
}
```

嗯，我们已经可以看到一些复杂的元素了。比如，为什么我使用了 `filter_map` 而不是 `filter`？ 这个烦人的 `pin!(stream)` 在做什么？ 我不会深入探讨这些问题。相反，这里有一些有用的链接：
- [Put a Pin on That](https://ohadravid.github.io/posts/2023-07-put-a-pin-on-that/)
- [How will futures::StreamExt::filter work with async closures?](https://www.reddit.com/r/rust/comments/r47iqi/how_will_futuresstreamextfilter_work_with_async/)

这次调查的目标是更好地理解这种管道的执行顺序、并发性和背压特性。例如，在上面的代码中，`map` 方法并发地执行了 `3` 次 `async_work`，但是如果 `async_predicate` 是一个耗时较长的操作呢？那么它会继续并发执行更多的 `async_work` 吗？
假设在完成了 `3` 次调用之后，它应该能够在 `async_predicate` 在后台运行的同时继续运行更多的任务，对吗？如果是这样的话，它会占用无限量的内存吗？
那 `filter_map` 方法会怎样呢？它没有明确的并发参数。它是串行运行提供的方法，还是以无限的并发运行？
文档中对这些问题并没有清晰的解释。我们需要亲自去看看实际的运行情况。

## 实验工具 - 可视化 Rust 流

我使用 [Bevy](https://bevyengine.org/) 来可视化在流管道中的数据流。想法涉及定义一个流通道，其中的方法通过一个频道报告它们的进度。我使用 `Bevy` 的 `EventWriter` 将这个信息发送给 `Bevy` 的渲染系统。

下面是它的样子:

<p align="center">
    <img src="./resources/buffer_1.gif">
</p>

在可视化中，我们看到每个流项目在管道的不同阶段进行导航的表示。工作单元从 *source* 开始，并移动到 `map(..).buffered(..)` 阶段。为了模拟真实世界的异步工作,我使用了一个小的 `sleep()` 调用循环。这代表了现实世界的场景，其中异步方法有多个 `await` 调用，并允许我们可视化未来的运行进度。

```rust
for i in 0..5 {
    tokio::time::sleep(duration / 5).await;
    tx.send(/* 更新 bevy 渲染系统 */).unwrap();
}
```

我们通过每个项目上的一个小进度条来可视化未来的进度。在一个项目完成 `buffered` 阶段后，它会继续前进到 `sink` 并完成它的旅程。

需要注意的是，这个可视化是从实际运行的 `Rust` 代码中获取的。这不是一个模拟，而是 `Rust` 流管道的实时可视化。

[你可以从这找到源码](https://github.com/alexpusch/rust-stream-vis).

## 实验一：[buffered](https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html#method.buffered)

```rust
stream::iter(0..10)
    .map(async_work)
    .buffered(5);
```

最多缓存 `n` 个 `future`，然后按照底层流的顺序返回输出。在任何时候，缓存中都不会超过 ` n` 个 `future`。

### 实验问题

- `buffered` 方法是在任何工作单元完成时就从源流获取新的工作单元，还是只有在最早的工作单元完成并进入到下一阶段时获取新的工作单元？

<p align="center">
    <img src="./resources/buffer_5.gif">
</p>

太棒了！看它多么顺畅！正如预期的那样，每个项目都经历了 `async_work`。`.buffered(5)` 步骤最多同时运行 `5` 个 `future`，并在他们的前置项也完成之前，保留已完成的 `future`。

### 实验结果
`buffered` 方法 **不会** 在任意一个项目完成后就获取新的工作单元。相反，它只有在最早的项目完成并进入下一阶段时才这样做。这是合理的。不同的行为会要求 `buffered` 方法存储无限数量的 `future` 的结果，这可能会导致内存使用过度。

我想知道是否有一种情况可以支持 `buffered_with_back_pressure(n: usize, b: usize)` 方法，它允许从源流中取出一些项目，最多 `b` 次。

## 实验二：[buffer_unordered](https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html#method.buffer_unordered)

```rust
stream::iter(0..10)
    .map(async_work)
    .buffer_unordered(5);
```

> 最多缓存 `n` 个 `future`，然后按照它们完成的顺序返回输出。在任何时候，缓存中都不会超过 `n` 个 `future`，不过可能会少于 `n` 个。

### 实验问题

- `buffer_unordered` 方法是在任何工作单元完成时就从源流获取新的工作单元，还是只有在最早的工作单元完成并进入到下一阶段时才获取新的工作单元？

<p align="center">
    <img src="./resources/buffer_unordered_5.gif">
</p>

与 `buffered` 不同，`buffer_unordered` 不会保留已完成的 `future`，而是在完成后立即进入下一阶段。

### 实验结果
`buffer_unordered` 方法 **确实** 会在任何工作单元完成后立即获取新的工作单元。与 `buffered` 不同， `unordered` 版本不需要保留已完成的 `future` 来维持输出顺序。这使它能以更高的吞吐量处理流。

## 实验三：[filter_map](https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html#method.filter_map)

```rust
stream::iter(0..10)
    .filter_map(async_predicate);
```

> 根据提供的异步筛选条件函数过滤这个流产生的值，并同时将它们映射到不同的类型。当这个流的值可用时，提供的函数将被运行。

### 实验问题

- `filter` 方法的执行特性是并行还是串行？

<p align="center">
    <img src="./resources/filter.gif">
</p>

### 实验结果

不出所料。`filter` 操作符是串行处理每个 `future` 的。

如果我们想要并发地完成异步过滤，我们可以使用 `map`、`buffered` 和 `filter_map(future::ready)` 的组合。`map().buffered()` 组合会并发地计算筛选条件函数，而 `filter_map` 则会从流中移除失败的项目。

```rust
stream::iter(0..10)
    .map(async_predicate)
    .buffered(5)
    .filter_map(future::ready); // ready 函数将返回被包装在 ready future 中的筛选条件函数的结果
```

## 实验四：buffered + filter_map

```rust
stream::iter(0..10)
    .map(async_work)
    .buffered(3)
    .filter_map(async_predicate);
```

### 实验问题

- 如果`filter_map` 步骤的运行时间很长，会如何影响 `buffered` 步骤的并发性呢？

<p align="center">
    <img src="./resources/buffer_filter_long.gif">
</p>

好吧，出乎意料！这个流的行为并不像我最初想象的那样。当 `async_predicate` 正在执行时，没有任何 `async_work` `future` 在进行。更进一步说，在第一批五个 `future` 完成之前，也没有新的 `future` 开始运行。这是怎么回事？

让我们看看当我们使用 `buffer_unordered` 替代 `buffered` 时发生了什么？

<p align="center">
    <img src="./resources/buffer_unordered_filter_long.gif">
</p>

情况基本相同。再次说明，在 `async_predicate` 完成之前， `async_work` `future` 都是被挂起的。

这会不会和 `filter_map` 有关呢？让我们尝试将两个 `buffered` 步骤串行放置：

<p align="center">
    <img src="./resources/buffer_buffer.gif">
</p>

不，行为仍然保持不变。

### 到底发生了什么？

原来我并不是第一个遇到这个困难的人。这是[Barbara 所面临的同样的问题](https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo/barbara_battles_buffered_streams.html)。


要真正理解发生了什么，我们需要对 Future、异步执行器和流接口有深入的理解。[The async book](https://rust-lang.github.io/async-book) 以及 fasterthanlime 的 [Understanding Rust futures by going way too deep](https://fasterthanli.me/articles/understanding-rust-futures-by-going-way-too-deep) 等资源可以作为良好的起点。

我会尽量给你一些直观的解释。

第一个线索来自于这个问题 - 什么时候 `Rust` 会并发运行两个 `future`？有 [join!](https://docs.rs/futures/latest/futures/macro.join.html) 和 [select!](https://docs.rs/futures/latest/futures/macro.select.html) 宏，以及 [spawn](https://docs.rs/tokio/latest/tokio/task/fn.spawn.html) 新的异步任务的能力。然而，流接口既不会对不同管道步骤创建的 `future` 进行 `join` 或 `select`，也不会在每次执行 `future` 时 `spawn` 新任务。

### 深入探究
让我们仔细看看我们的示例，并尝试分析控制流。

```rust
let stream = stream::iter(0..10)
    .map(async_work)
    .buffered(5) 
    .filter_map(async_predicate);

pin!(stream);

while let Some(next) = stream.next().await {
    println!("finished working on: {}", next);
}
```

首先我们创建了流实例。在 `Rust` 中，`Future` 在被 `await` 之前是不会执行的。因此，示例的第一行没有独立的效果。

让我们看看 `stream` 变量的类型定义：
```rust
FilterMap<
  Buffered<Map<Iter<Range<i32>>, fn async_work(i32) -> impl Future<Output = i32>>>,
  impl Future<Output = Option<i32>>,
  fn async_predicate(i32) -> impl Future<Output = Option<i32>
>
```

震惊，我们发现了一个有五层嵌套的结构体，嵌套关系从里到外以此为：`Range`、`Iter`、`Map`、`Buffered` 和 `Filter`。

这些结构体类型被称为 **适配器**。每个适配器都持有状态和数据，并实现了某些特性，在我们的例子中是 `Stream`。它们将自己的逻辑包装在这个特性周围。

例如，[`Buffered` 适配器](https://github.com/rust-lang/futures-rs/blob/master/futures-util/src/stream/stream/buffered.rs) 拥有一个源 `stream` 和 `in_progress_queue: FuturesOrdered` 来管理缓冲。

优雅地跳过 `pin!`。

那么，在第一个 `stream.next().await` 命令上会发生什么呢？[`Next` future](https://github.com/rust-lang/futures-rs/blob/master/futures-util/src/stream/stream/next.rs#L32) 调用 `stream.poll_next_unpin(cx)`，其中 `stream` 是 `FilterMap` 的一个实例。

反过来，`FilterMap::poll_next` 的实现是[轮询](https://github.com/rust-lang/futures-rs/blob/master/futures-util/src/stream/stream/filter_map.rs#L79)它的内部流 - `Buffered` 流 - 并在结果上执行 `async_predicate`。
`Buffered::poll_next` 方法[轮询](https://github.com/rust-lang/futures-rs/blob/master/futures-util/src/stream/stream/buffered.rs#L70)它的内部流最多 `max` 次，直到内部缓冲区被填满。

对于每一次这样的轮询，`Map` 流从它的源流中[获取一个项目](https://github.com/rust-lang/futures-rs/blob/master/futures-util/src/stream/stream/map.rs#L58)，并运行返回一个` future` 的 `async_work` 方法。

注意 `future` 被并行执行的唯一地方是在 `Buffered::poll_next` 实现中的 `FuturesOrdered` 示例。

我们可以大致将这个示例转换为下面的伪代码：

```rust
let range_stream = stream::iter(0..10);
let in_progress_queue = FuturesOrdered::new()

loop {
    // 最多将 5 个项目缓存到队列中
    while in_progress_queue.len() < 5 {
        // 从原流中获取值，并在值上运行 map 步骤
        let next = range_stream.next();
        // 注意我们还没有 await 返回的 future
        let future = async_work(next);

        in_progress_queue.push(future)
    }

    // 执行缓存的 future。获取下一个完成的 future （保持顺序）
    // 这是 5 个 future 并行运行
    let next = in_progress_queue.poll_next().await;
    
    // 过滤结果
    // 在 `in_progress_queue` 中的 future 没有被轮询
    let predicate_result = async_predicate(next).await;

    // 相应地产生结果
}
```

当将流管道分解为这种简单的表示时，我们实验的结果就变得很清晰了。在执行 `async_predicate` 时，我们没有轮询 `in_progress_queue` - 因此 `future` 是“卡住”的。
此外，当 `async_predicate` 完成时，我们返回并从 `in_progress_queue` 轮询新的 `future`。但是，即使我们成功了，后续的 `in_progress_queue.poll_next().await` 也只会运行一小段时间 - 直到正在进行的 `future` 完成为止。这给新轮询的 `future` 执行的时间非常有限。事实上，根据可视化，它们可能根本就没有被轮询。一旦初始批次的 `future` 完成，新轮询的 `future` 就有机会执行。

此时，你们中的一些人可能会对结果产生怀疑。当然，如果你发起了一个 `100ms` 的网络请求，它仍然需要 `100ms` 才能完成，无论托管的异步执行器是什么。这当然是正确的。一旦 `future` 被轮询，底层实现就会运行到完成，并耐心地等待再次被轮询。我描述的这种效果会导致这最终的轮询被延迟。

为了说明这种效果，以下两个版本的 `async_work` 在流管道中会有非常不同的运行特性。

第一个版本有一个单独的 `tokio::time:sleep(100ms)` 调用。`sleep()` 返回 [`Sleep`](https://docs.rs/tokio/latest/tokio/time/struct.Sleep.html)，它直接实现了 `Future`。这意味着 `async_work` 的第一次轮询将反过来调用 `Sleep::poll`，它将执行所需的操作来睡眠 `100ms`。但是，无论这个 `future` 何时被轮询，它都会报告它已经 `Ready`，并且 `async_work` 将返回。

```rust
async fn async_work(x: i32) -> i32 {
    sleep(Duration::from_millis(100)).await;

    x
}
```

第二个版本有 `5` 个 `sleep(20ms)` 调用。在这种情况下，每个后续的 `.await` 可能会一次又一次地遭受轮询延迟的影响。这就是我们在这个调查中可视化的 `future` 的情况，也可能是更好地模拟现实世界用例的方式。

```rust
async fn async_work(x: i32) -> i32 {
    sleep(Duration::from_millis(20)).await;
    sleep(Duration::from_millis(20)).await;
    sleep(Duration::from_millis(20)).await;
    sleep(Duration::from_millis(20)).await;
    sleep(Duration::from_millis(20)).await;

    x
}
```

## 实验总结

我们的实验揭示了**流接口**管道可能出现令人惊讶的次优表现。如果简单地看一个管道，我们可能会想象一切都在并发运行。然而，现实并不符合这些期望。

你应该使用**流接口**吗？与我们行业中的许多其他事物一样，这取决于权衡结果。一方面，这个接口允许我们快速满足需求，并提供一个清晰优雅的接口。另一方面，管道吞吐量将不会是最优的。

在我看来，在许多情况下，放弃这个接口可能会被视为过早的优化。尽管如此，这些发现绝对值得你考虑。