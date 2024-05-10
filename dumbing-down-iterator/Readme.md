# Dumbing down Rust Iterator internals

<details>
  <summary>Additional languages</summary>
  <ul>
    <li>
      <a href='https://github.com/yushengguo557/rust-magic-patterns/blob/translation-zh-cn/dumbing-down-iterator/Readme_ZH_CN.md'>Simplified Chinese</a> - <a href="https://github.com/yushengguo557">@yushengguo557</a>
    </li>
  </ul>
</details>

The Rust Iterator API is one of the first things a Rust novice should learn after familiarizing themselves with the basics of the language. However, this API and its documentation can be daunting for a beginner.

For example, have you ever looked at the [map](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.map) method documentation and wondered why it's so complicated?

```rust
fn map<B, F>(self, f: F) -> Map<Self, F> 
where
    Self: Sized,
    F: FnMut(Self::Item) -> B,
```

What's B and F? Why does this method return a `Map` type? 

Another example is the `collect` method that somehow allows us to convert an iterator into various types of collections:

```rust
let v: Vec<_> = (0..10).collect();
let s: HashSet<_> = (0..10).collect();
```

How can this be possible? Rust is a statically typed language, right? How can the same method return different types?

Lets learn how the Iterator API type system works by dumbing it down to a bare minimum implementation.

## Iterator trait
The [Iterator trait](https://doc.rust-lang.org/std/iter/trait.Iterator.html) is the first building block of the iterator API.

Let's define a simple MyIterator trait to demonstrate its structure:

```rust
pub trait MyIterator {
    type Item;

    fn next(&mut self) -> Option<Self::Item>;
}
```

This trait defines the basic mechanism for iteration. It has a single method `next` which returns the next item in the iteration, or `None` if the iteration is over. The `Item` [associated type](https://doc.rust-lang.org/book/ch19-03-advanced-traits.html#specifying-placeholder-types-in-trait-definitions-with-associated-types) defines the type of the items in the iteration.

## Example - SliceIterator
Let's implement a simple iterator for a simple slice - `[T]`. This is a 
dumbing down of [std::slice::Iter](https://doc.rust-lang.org/std/slice/struct.Iter.html)

The `SliceIterator` struct holds a reference to the vector and the current position in the iteration. 

```rust
/// The lifetime parameter `'a` is used to ensure that the iterator does not outlive the data it iterates over.
pub struct SliceIterator<'a, T> {
    data: &'a [T],
    pos: usize,
}

impl<'a, T> SliceIterator<'a, T> {
    // pub(crate) is used to make this constructor visible only to this crate
    pub(crate) fn new(data: &'a [T]) -> Self {
        SliceIterator { data, pos: 0 }
    }
}

impl<'a, T> MyIterator for SliceIterator<'a, T> {
    /// The Item type of SliceIterator is a reference to the type of the slice.
    /// The lifetime parameter `'a` ensures that the returned reference does not outlive the
    /// data we are iterating over.
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
Let's test our iterator:

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

**Note the lifetime parameter `'a`** which is used to ensure that the iterator does not outlive the slice it iterates over. This is important because the iterator holds a reference to the slice, and if the slice is dropped before the iterator, the iterator will hold a dangling reference. We don't want that.

Thanks to this lifetime parameter, the following code will not compile:
```rust
let data = vec![1, 2, 3, 4, 5];             // ──| data variable lifetime 'a starts
let mut iter = SliceIterator::new(&data);   //   | SliceIterator::new is bound to 'a lifetime
drop(data);                                 // __| data is dropped - lifetime ends
iter.next();                                // here we violate the 'a constraint defined in SliceIterator
```

## Iterator methods
The [iterator trait](https://doc.rust-lang.org/std/iter/trait.Iterator.html) has a single required method `next`, but it also has many provided methods. For example, the previously mentioned `map` method.

```rust
fn map<B, F>(self, f: F) -> Map<Self, F> 
where
    Self: Sized,
    F: FnMut(Self::Item) -> B,
```

The `map` method takes a closure and returns a `Map` struct. The `Map` struct is an intermediate utility struct which implements the `Iterator` trait and holds any needed state and logic for this particular iterator method. This concept is often called an [Adapter](https://doc.rust-lang.org/std/iter/index.html#adapters)

Let's expand the `MyIterator` trait to include `map` and `filter` methods:

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

### Map Adapter
This is a simplification of [std::iter::Map](https://doc.rust-lang.org/std/iter/struct.Map.html)

```rust
/// I is the iterator type the map method is applied on.
/// F is the type of the closure that is used to map the elements. It is constrained to be of type
/// `FnMut(Self::Item) -> B` in the MyIterator implementation
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
    /// The Item type of MyMap is the type returned by the map closure
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

Lets examine the various generic variables we used here:

`I` - The MyMap struct owns the instance of the iterator we're mapping over. `I` is the type of this iterator. For example, if we are mapping over a `SliceIterator`, `I` will be `SliceIterator`.

`F` - The MyMap struct also owns the closure we use to map the items. `F` is the type of this closure. For example, if the closure is `|x| x * 2`, `F` will be `FnMut(&i32) -> i32`.

`B` - The map function return type needs to be defined as well. The return value of the map function is the item type of the MyMap iterator. For example, if the closure is `|x| x * 2`, `B` will be `i32`, and the MyMap iterator `next` method have `Option<i32>` as its return value.

### Filter Adapter
Dumbing down of [std::iter::Filter](https://doc.rust-lang.org/std/iter/struct.Filter.html)

```rust
/// I is the iterator type the filter method is applied on.
/// P is the type of the predicate function we invoke on each item
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
    /// The Item type of MyFilter is the same as the Item type of the inner iterator
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        // we iterate over the inner iterator until we find an item that passes the filter
        while let Some(x) = self.iter.next() {
            if (self.filter_fn)(&x) {
                return Some(x);
            }
        }

        None
    }
}
```

Adapters like `Map` and `Filter` are internal to the iterator implementation, so you will rarely use them directly, nonetheless, let's see a usage example:

```rust
#[test]
fn my_map_filter_next_returns_next_item() {
    let iter = SliceIterator::new(&[1, 2, 3]);
    // note that the predicate closure is a reference to a reference. Check out the code to figure out why!
    let filter = MyFilter::new(iter, |x: &&i32| **x % 2 == 0);
    let mut map = MyMap::new(filter, |x| x * 2);

    assert_eq!(map.next(), Some(4));
    assert_eq!(map.next(), None);
}
```

## Collect
Most iterator usages will end with a call to the [collect](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.collect) method in order to collect the iterated items into a collection of a concrete type. Let's get under the hood of this one as well.

First, we expand the `MyIterator` trait to include the `collect` method:

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
The `collect` method defines a return position generic type `B` which must implement the `MyFromIterator` trait. 


## FromIterator trait
[std::iter::FromIterator](https://doc.rust-lang.org/std/iter/trait.FromIterator.html) is a trait that defines how to create a collection from an iterator. Previously, we defined the `SliceIterator` struct which converts a vector into an iterator. Now we need to define the opposite - a way to convert an iterator into a collection.

Let's define the `MyFromIterator` trait:

```rust
pub trait MyFromIterator<T> {
    fn my_from_iter<I>(iter: I) -> Self
    where
        I: MyIterator<Item = T>;
}
```
`my_from_iter` allows us to convert anything that implements `MyIterator` into the trait implementing collection. We could use this to convert `SliceIterator`, `MyMap`, `MyFilter`, and any other `MyIterator` into a collection.

Note that for simplicity, we are not using the `IntoIterator` trait here, as used in the actual `std::iter::FromIterator` trait.

For example, let's implement `MyFromIterator` for `Vec<T>` and `HashSet<T>`. Both examples iterate over the target iterator and push each item into the constructed collection.

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
    // we need to add these constraints because HashSet::new() requires them
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

Let's look at the `collect` usage example again:
```rust
let iter = SliceIterator::new(&[1, 2, 3]);
let v: Vec<_> = iter.collect();
```

With our simplified implementation, it actually expands to

```rust
fn collect(self) -> Vec<u64>
{
    Vec::<u64>::my_from_iter(self)
}   
```

and more specifically

```rust
Vec::<u64>::my_from_iter(iter);
```

Any other collection type that implements `MyFromIterator` can be used in the same way:

```rust
let s: HashSet<_> = iter.collect();
let b: BTreeSet<_> = iter.collect();
let l: LinkedList<_> = iter.collect();
```
Each one of these `collect` calls actually expands to a different `my_from_iter` implementation.

```rust
let s = HashSet::<_>::my_from_iter(iter);
let b = BTreeSet::<_>::my_from_iter(iter);
let l = LinkedList::<_>::my_from_iter(iter);
```

Now we know that `collect` is a thin sugar coating over `FromIterator` and `from_iter`, and in fact, does not violate Rust's strict type system.

## Dumbing up

Be sure to go over the [std::iter](https://doc.rust-lang.org/std/iter/index.html) documentation to see all the other iterator methods and adapters.

All the source code for this article can be found in the [src](./src/my_iterator.rs) directory.