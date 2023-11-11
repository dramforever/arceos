use core::{fmt::*, iter::*};

#[derive(Clone, Copy)]
pub(crate) struct DebugIter<I: ?Sized>(I);

pub(crate) trait IteratorDebug {
    fn debug(self) -> DebugIter<Self>;
}

impl<I: Iterator> IteratorDebug for I {
    fn debug(self) -> DebugIter<Self> {
        DebugIter(self)
    }
}

impl<I: Iterator> Iterator for DebugIter<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<I: FusedIterator> FusedIterator for DebugIter<I> {}

impl<I: DoubleEndedIterator> DoubleEndedIterator for DebugIter<I> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<I: Iterator + Clone> Debug for DebugIter<I>
where
    I::Item: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_list().entries(self.clone()).finish()
    }
}
