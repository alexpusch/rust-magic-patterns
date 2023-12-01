use crate::{MyFilter, MyFromIterator, MyMap};

/// Main iterator trait. This trait defines how a type can be iterated over.
/// This is a dumbing down of the `Iterator` trait from the standard library.
/// https://doc.rust-lang.org/std/iter/trait.Iterator.html
pub trait MyIterator {
    /// The type of the elements being iterated over. This is used by the
    /// iterator functions to describe the type of the elements being returned.
    type Item;

    fn next(&mut self) -> Option<Self::Item>;

    fn collect<B>(self) -> B
    where
        B: MyFromIterator<Self::Item>,
        // the Sized bound is required since we are passing self as an argument, and Rust must
        // know the size of all arguments at compile time
        Self: Sized,
    {
        B::my_from_iter(self)
    }

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
