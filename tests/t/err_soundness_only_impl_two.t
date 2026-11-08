// Two impls could be the one a call is of: neither decides what's not
// known of its receiver's type.
#[derive(Discard)]
struct W<T> { v: u8, t: Option<T> }

impl<T> W<T> {
    fn new() -> W<T> { W { v: 0, t: None } }
}

trait Get {
    fn get(&self) -> u8;
}

impl Get for W<u8> {
    fn get(&self) -> u8 { self.v }
}

impl Get for W<u16> {
    fn get(&self) -> u8 { self.v + 1 }
}

fn main() {
    let w = W::new();
    let _ = w.get();
}
// error: cannot infer `T`
