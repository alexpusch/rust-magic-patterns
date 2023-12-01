mod my_filter;
mod my_from_iterator;
mod my_iterator;
mod my_map;
mod slice_iterator;

pub use my_filter::*;
pub use my_from_iterator::*;
pub use my_iterator::*;
pub use my_map::*;
pub use slice_iterator::*;

#[cfg(test)]
mod test {
    use crate::{MyIterator, SliceIterator};

    #[test]
    fn test_my_iterator() {
        let result = SliceIterator::new(&[1, 2, 3, 4, 5])
            .filter(|x| *x % 2 == 0)
            .map(|x| x * 2)
            .collect::<Vec<_>>();

        assert_eq!(result, vec![4, 8]);
    }
}
