use crate::MyIterator;

/// An example MyIterator over a slice of T
/// This is a dumbing down of the `std::slice::Iter` iterator from the standard library.
/// https://doc.rust-lang.org/std/slice/struct.Iter.html
///
/// The lifetime parameter `'a` is used to ensure that the iterator does not outlive the data
/// it is iterating over.
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

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.data.len() {
            None
        } else {
            let result = Some(&self.data[self.pos]);
            self.pos += 1;
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slice_iterator_next_returns_next_item() {
        let mut iter = SliceIterator::new(&[1, 2, 3]);
        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter.next(), Some(&3));
    }

    #[test]
    fn slice_iterator_next_returns_none_when_iteration_is_over() {
        let mut iter = SliceIterator::new(&[1]);
        _ = iter.next();
        assert_eq!(iter.next(), None);
    }
}
