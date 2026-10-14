//@ charon-args=--remove-associated-types=*
//! An impl's implied clause is proved by an impl expr whose regions trait resolution erased:
//! `impl De for IterMut<'a, T>` proves `IterMut<'a, T>: Iterator` by
//! `impl_Iterator_for_IterMut<'_, T>`, so the `Item` lifted into `De`'s generics read
//! `&'_ mut T`. Matching that impl's header against the clause recovers `'a`.
pub trait De: Iterator {
    fn next_back(&mut self) -> Option<Self::Item>;
}

impl<'a, T> De for core::slice::IterMut<'a, T> {
    fn next_back(&mut self) -> Option<&'a mut T> {
        DoubleEndedIterator::next_back(self)
    }
}

pub fn last_mut(a: &mut [u32]) -> Option<&mut u32> {
    De::next_back(&mut a.iter_mut())
}
