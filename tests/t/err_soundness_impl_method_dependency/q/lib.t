//! A trait whose method takes any iterator of `u8`s.

export trait Run {
    fn run<I: Iterator<Item = u8> + Discard>(i: I) -> u8 with I::e;
}
