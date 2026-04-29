pub trait Iter {
    type Item;
    fn peek(&self) -> Option<&Self::Item>;
    fn peek_behind(&self) -> Option<&Self::Item>;
    fn peek_ahead(&self) -> Option<&Self::Item>;
    fn next(&mut self) -> Option<&Self::Item>;
}
