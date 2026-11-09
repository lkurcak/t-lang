// The result is a `&mut Option<T>`: what's written through it can be a
// `None`, which changes which variant the field is. That's not a value of
// a type parameter, so the method can't promise `has()` stays.
struct Slot<T> {
    o: Option<T>,
}

impl<T> Slot<T> {
    #[measure]
    fn has(&self) -> bool { self.o.is_some() }

    fn whole(&mut self) -> &mut Option<T> ensures self.has() == old(self.has()) { &mut self.o }
}

fn main() {
    let mut s = Slot { o: Some(1u8) };
    if s.has() {
        *s.whole() = None;
        let h: { h: bool | h } = s.has();
        let _ = h;
    }
}
// error: `ensures` can't name `self` at return
