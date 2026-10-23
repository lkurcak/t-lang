// Only `core` gives `str` methods of its own: they keep its bytes UTF-8.
impl str {
    fn second(&self) -> Option<&u8> { self.as_bytes().get(1) }
}

fn main() {}
// error: an inherent impl can only be for a struct or enum, not `str`; to give `str` methods, declare a trait and implement that
