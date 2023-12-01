use crate::MyIterator;

/// An iterator that applies a function to each item.
/// This is a dumbing down of the `Map` iterator from the standard library.
/// https://doc.rust-lang.org/std/iter/struct.Map.html
///
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

/// We introdcue a new genetric paramter B for the return type of the map closure
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

#[cfg(test)]
mod tests {
    use crate::SliceIterator;

    use super::*;

    #[test]
    fn my_map_next_returns_next_item() {
        let mut iter = MyMap::new(SliceIterator::new(&[1, 2, 3]), |x| x * 2);
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(4));
        assert_eq!(iter.next(), Some(6));
    }

    #[test]
    fn my_map_next_returns_none_when_iteration_is_over() {
        let mut iter = MyMap::new(SliceIterator::new(&[1]), |x| x * 2);
        _ = iter.next();
        assert_eq!(iter.next(), None);
    }
}
