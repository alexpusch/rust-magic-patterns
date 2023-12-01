use crate::MyIterator;

/// An iterator that filters the elements of another iterator.
/// This is a dumbing down of the `Filter` iterator from the standard library.
/// https://doc.rust-lang.org/std/iter/struct.Filter.html
///
/// P (for predicate) is the type of the closure that is used to filter the elements. It
/// is contrained to be of type `FnMut(&Self::Item) -> bool` in the MyIterator implementation
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
    /// The Item type of MyFilter is the same as the Item type of the underlying iterator
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        // iterate over the iterator until we find an element that matches the filter
        while let Some(x) = self.iter.next() {
            if (self.filter_fn)(&x) {
                return Some(x);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use crate::SliceIterator;

    use super::*;

    #[test]
    fn my_filter_next_returns_next_item() {
        let mut iter = MyFilter::new(SliceIterator::new(&[1, 2, 3, 4]), |x: &&i32| **x % 2 == 0);
        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter.next(), Some(&4));
    }

    #[test]
    fn my_filter_next_returns_none_when_iteration_is_over() {
        let mut iter = MyFilter::new(SliceIterator::new(&[1, 2]), |x: &&i32| **x % 2 == 0);
        _ = iter.next();
        assert_eq!(iter.next(), None);
    }
}
