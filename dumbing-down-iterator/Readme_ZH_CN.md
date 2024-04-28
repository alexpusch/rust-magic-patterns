# 简化 Rust 迭代器内部工作原理

Rust 迭代器 API 是新手学习 Rust 语法基础后应该学习的第一个内容之一。但是，这个 API 及其文档对初学者来说可能很吓人。

比如，你是否曾看过 [map](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.map) 方法的文档，并感到它很复杂？

```rust
fn map<B, F>(self, f: F) -> Map<Self, F> 
where
    Self: Sized,
    F: FnMut(Self::Item) -> B,
```
`B` 和 `F` 是什么？为什么这个方法返回一个 `Map` 类型？

另一个例子是 `collect` 方法，它可以让我们将迭代器转换为各种类型的集合:

```rust
let v: Vec<_> = (0..10).collect();
let s: HashSet<_> = (0..10).collect();
```

这怎么可能呢？Rust 是静态类型语言啊！一个方法怎么能返回不同类型呢？

让我们通过实现迭代器 API 的最小化版本，来学习它的类型系统是如何工作的。

## 迭代器特征 (Iterator trait)

迭代器特征 ([Iterator trait](https://doc.rust-lang.org/std/iter/trait.Iterator.html)) 是迭代器 API 的基础模块。

让我们定义一个简单的 `MyIterator` 特征来演示它的结构：

```rust
pub trait MyIterator {
    type Item;

    fn next(&mut self) -> Option<Self::Item>;
}
```

这个特征定义了迭代的基本机制。它有一个 `next` 方法，返回迭代中的下一个项目，或者当迭代结束时返回 `None`。`Item` 关联的类型定义了迭代中所有项目的类型。

## 示例 - 切片迭代器 (SliceIterator)
让我们为一个简单的切片实现一个简单的迭代器 - `[T]`。这是对标准库 [std::slice::Iter](https://doc.rust-lang.org/std/slice/struct.Iter.html) 的简化。

`SliceIterator` 结构体持有向量的引用和当前迭代的位置。

```rust
/// 生命周期参数 `'a` 用于确保迭代器不会超过它迭代的数据的生命周期
pub struct SliceIterator<'a, T> {
    data: &'a [T],
    pos: usize,
}

impl<'a, T> SliceIterator<'a, T> {
    // pub(crate) 用于使这个构造函数只在这个crate内可见
    pub(crate) fn new(data: &'a [T]) -> Self {
        SliceIterator { data, pos: 0 }
    }
}

impl<'a, T> MyIterator for SliceIterator<'a, T> {
    /// SliceIterator 的 Item 类型是一个对切片元素类型的引用
    /// 生命周期参数 `'a` 可以保证返回的引用不会超过我们正在迭代的数据的生命周期
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item>
    {
        if self.pos >= self.data.len() {
            None
        } else {
            let result = Some(&self.data[self.pos]);
            self.pos += 1;
            result
        }
    }
}
```
让我们测试我们的迭代器

```rust
#[test]
fn slice_iterator_next_returns_next_item() {
    let mut iter = SliceIterator::new(&[1, 2, 3]);
    assert_eq!(iter.next(), Some(&1));
    assert_eq!(iter.next(), Some(&2));
    assert_eq!(iter.next(), Some(&3));
    assert_eq!(iter.next(), None);
}
```

**请注意生命周期参数 `'a'` **，它用于确保迭代器不会超过它迭代的切片的生命周期。这一点很重要，因为迭代器持有切片的引用，如果切片在迭代器之前被释放，迭代器就会持有一个无效的引用。我们不希望发生这种情况。

由于声明周期参数的使用，下面的代码不会被编译：
```rust
let data = vec![1, 2, 3, 4, 5];             // ──| data 变量的生命周期 'a 开始
let mut iter = SliceIterator::new(&data);   //   | SliceIterator::new 被限定为 'a 生命周期
drop(data);                                 // __| data 被释放 - 生命周期结束
iter.next();                                // 在这里，我们违反了 SliceIterator 定义的 'a 限制
```

## 迭代器方法 [Iterator methods]
迭代器特征 ([iterator trait](https://doc.rust-lang.org/std/iter/trait.Iterator.html)) 除了一个必须的 `next` 方法，还提供了许多其他方法。例如前面提到的 `map` 方法。

```rust
fn map<B, F>(self, f: F) -> Map<Self, F> 
where
    Self: Sized,
    F: FnMut(Self::Item) -> B,
```

`map` 方法接收一个闭包并返回一个 `Map` 结构体。`Map` 结构体是一个中间的实用结构体，它实现了迭代器特征并持有这个特定迭代器方法需要的任何状态和逻辑。这个概念经常被称为适配器 ([Adapter](https://doc.rust-lang.org/std/iter/index.html#adapters))。


让我们扩展 `MyIterator` 特征，增加 `map` 和 `filter` 方法：

```rust
pub trait MyIterator {
    type Item;

    // ...

    fn map<B, F>(self, map_fn: F) -> MyMap<Self, F>
    where
        Self: Sized,
        F: FnMut(Self::Item) -> B,
    {
        MyMap::new(self, map_fn)
    }

    fn filter<P>(self, filter_fn: P) -> MyFilter<Self, P>
    where
        Self: Sized,
        P: FnMut(&Self::Item) -> bool,
    {
        MyFilter::new(self, filter_fn)
    }
}
```

### Map 适配器 (Map Adapter) 

这是标准库 [std::iter::Map](https://doc.rust-lang.org/std/iter/struct.Map.html) 的简化

```rust
/// I 是 map 方法应用的迭代器的类型
/// F 是用于映射元素的闭包的类型。在 MyIterator 的实现中，它被限定为：
/// `FnMut(Self::Item) -> B`
pub struct MyMap<I, F>
where
    I: MyIterator,
{
    iter: I,
    map_fn: F,
}

impl<I, F> MyMap<I, F>
where
    I: MyIterator,
{
    pub(crate) fn new(iter: I, map_fn: F) -> Self {
        MyMap { iter, map_fn }
    }
}

impl<B, I, F> MyIterator for MyMap<I, F>
where
    I: MyIterator,
    F: FnMut(I::Item) -> B,
{
    /// MyMap 迭代器的 Item 类型是映射闭包返回的类型
    type Item = B;

    fn next(&mut self) -> Option<B> {
        if let Some(x) = self.iter.next() {
            Some((self.map_fn)(x))
        } else {
            None
        }
    }
}
```

让我们来研究我们在这里使用的各种泛型变量:

`I` - MyMap 结构体拥有我们映射的迭代器的实例。`I` 是这个迭代器的类型。例如，如果我们映射的是 `SliceIterator`，那么 `I` 将是 `SliceIterator`。

`F` - MyMap 结构体也拥有我们用于映射项的闭包。`F` 是这个闭包的类型。例如，如果闭包是 `|x| x * 2`，那么 `F` 将是 `FnMut(&i32) -> i32`。

`B` - `map` 函数的返回类型也需要定义。`map` 函数的返回值就是 `MyMap` 迭代器的项类型。例如，如果闭包是 `|x| x * 2`，那么 `B` 将是 `i32`，`MyMap` 迭代器的 `next` 方法的返回值将是 `Option<i32>`。

### 过滤适配器 (Filter Adapter)
[std::iter::Filter](https://doc.rust-lang.org/std/iter/struct.Filter.html) 的简化版本。

```rust
/// I 是过滤方法应用的迭代器类型
/// P 是我们对每个项目调用的判断函数的类型
pub struct MyFilter<I, P>
where
    I: MyIterator,
{
    iter: I,
    filter_fn: P,
}

impl<I, P> MyFilter<I, P>
where
    I: MyIterator,
{
    pub(crate) fn new(iter: I, filter_fn: P) -> Self {
        MyFilter { iter, filter_fn }
    }
}

impl<I, P> MyIterator for MyFilter<I, P>
where
    I: MyIterator,
    P: FnMut(&I::Item) -> bool,
{
    /// MyFilter 迭代器的 Item 类型与内部迭代器的 Item 类型相同
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        // 我们会迭代内部迭代器，直到找到一个通过过滤条件的项目
        while let Some(x) = self.iter.next() {
            if (self.filter_fn)(&x) {
                return Some(x);
            }
        }

        None
    }
}
```

类似 Map 和 Filter 这样的适配器是迭代器内部实现的，所以我们很少会直接使用它们。不过，让我们看一个使用例子：

```rust
#[test]
fn my_map_filter_next_returns_next_item() {
    let iter = SliceIterator::new(&[1, 2, 3]);
    // 需要注意的是，判断闭包的参数类型是一个对引用的引用。请检查代码来理解为什么
    let filter = MyFilter::new(iter, |x: &&i32| **x % 2 == 0);
    let mut map = MyMap::new(filter, |x| x * 2);

    assert_eq!(map.next(), Some(4));
    assert_eq!(map.next(), None);
}
```

## Collect
大多数迭代器的使用会以对 [collect](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.collect) 方法的调用结束，目的是将迭代得到的项目收集到一个具体类型的集合中。让我们也来分析 `collect` 方法的内部实现。

首先，我们扩展 `MyIterator` 特征来包含 `collect` 方法:

```rust
pub trait MyIterator {
    // ...

    fn collect<B>(self) -> B
    where
        B: MyFromIterator<Self::Item>,
        Self: Sized,
    {
       B::my_from_iter(self)
    }
}
```

`collect` 方法定义了一个必须实现 `MyFromIterator` 特性的泛型类型的返回值 `B`。

## FromIterator 特征

[std::iter::FromIterator](https://doc.rust-lang.org/std/iter/trait.FromIterator.html) 是一个定义如何从迭代器中创建集合的特征。之前，我们定义了 `SliceIterator` 结构体，它可以将向量转换为迭代器。现在我们需要定义相反的操作 - 将迭代器转换为集合。

让我们定义 `MyFromIterator` 特征：

```rust
pub trait MyFromIterator<T> {
    fn my_from_iter<I>(iter: I) -> Self
    where
        I: MyIterator<Item = T>;
}
```

`my_from_iter` 允许我们将任何实现了 `MyIterator` 特性的类型转换为实现了集合特性的类型。我们可以使用它来将 `SliceIterator`、`MyMap`、`MyFilter` 以及任何其他 `MyIterator` 转换为集合。

需要注意的是，为了简单，我们这里没有使用 `std::iter::FromIterator` 特性中实际使用的 `IntoIterator`特性。

例如，我们可以为 `Vec<T>` 和 `HashSet<T>` 实现 `MyFromIterator`特性。两种情况下的实现都会遍历目标迭代器，并将每个项目插入正在构建的集合中。

```rust
impl<T> MyFromIterator<T> for Vec<T> {
    fn my_from_iter<I>(mut iter: I) -> Self
    where
        I: MyIterator<Item = T>
    {
        let mut vec = Vec::new();

        while let Some(x) = iter.next() {
            vec.push(x);
        }

        vec
    }
}

impl<T> MyFromIterator<T> for HashSet<T>
where
    // 因为 HashSet::new() 需要它们，所以我们必须添加这些约束
    T: Eq + Hash,
{
    fn my_from_iter<I>(mut iter: I) -> Self
    where
        I: MyIterator<Item = T>,
    {
        let mut set = HashSet::new();
        while let Some(x) = iter.next() {
            set.insert(x);
        }

        set
    }
}
```

让我们再看一下 `collect` 方法的使用：

```rust
let iter = SliceIterator::new(&[1, 2, 3]);
let v: Vec<_> = iter.collect();
```

由于我们给出的实现是简化的，所以它实际会扩展为：

```rust
fn collect(self) -> Vec<u64>
{
    Vec::<u64>::my_from_iter(self)
}   
```

更具体的是

```rust
Vec::<u64>::my_from_iter(iter);
```

任何其他实现了 `MyFromIterator` 特征的集合类型都可以用同样的方式使用:

```rust
let s: HashSet<_> = iter.collect();
let b: BTreeSet<_> = iter.collect();
let l: LinkedList<_> = iter.collect();
```

其中每一个 `collect` 方法的调用实际上都会扩展成不同的 `my_from_iter` 的实现。 

```rust
let s = HashSet::<_>::my_from_iter(iter);
let b = BTreeSet::<_>::my_from_iter(iter);
let l = LinkedList::<_>::my_from_iter(iter);
```

现在我们知道，`collect` 实际上是 `FromIterator` 和 `from_iter` 的一层简单封装，而它本身实际上没有违反 Rust 严格的类型系统。

## 简单概述

请务必查看 [std::iter](https://doc.rust-lang.org/std/iter/index.html) 文档，了解所有其他迭代器方法和适配器。

本文所有源代码都可以在 [src](./src/my_iterator.rs) 目录下找到。
