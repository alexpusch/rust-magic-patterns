use std::collections::HashSet;

use crate::MyIterator;

/// Defines how a type can be created from an iterator.
/// This is a dumbing down of the `FromIterator` trait from the standard library.
/// https://doc.rust-lang.org/std/iter/trait.FromIterator.html
pub trait MyFromIterator<T> {
    fn my_from_iter<I>(iter: I) -> Self
    where
        // note that for simplicity I replaced `where T: IntoIterator<Item = A>` with a simpler condition
        I: MyIterator<Item = T>;
}

impl<T> MyFromIterator<T> for Vec<T> {
    fn my_from_iter<I>(mut iter: I) -> Self
    where
        I: MyIterator<Item = T>,
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
    T: Eq + std::hash::Hash,
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

#[cfg(test)]
mod tests {
    use crate::SliceIterator;

    use super::*;

    #[test]
    fn vec_from_iter_returns_vec() {
        let iter = SliceIterator::new(&[1, 2, 3]);
        let result = Vec::my_from_iter(iter);
        assert_eq!(result, vec![&1, &2, &3]);
    }

    #[test]
    fn hash_set_from_iter_returns_hash_set() {
        let iter = SliceIterator::new(&[1, 2, 3]);
        let result = HashSet::my_from_iter(iter);
        assert_eq!(result, HashSet::from([&1, &2, &3]));
    }
}
