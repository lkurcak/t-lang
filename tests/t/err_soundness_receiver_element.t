// A method that takes `&mut self`, called on a refined element
// (`a[0].poke()`, a `&mut` to `*a.index_mut(0)` the method call makes):
// what it may write there must keep what's declared of the element.
#![allow(dead_code)]
trait Poke {
    fn poke(&mut self);
}

impl Poke for usize {
    fn poke(&mut self) { *self = 99 }
}

fn array(a: &mut [{ t: usize | t < 5 }; 2]) { a[0].poke() }

fn main() {}
// error: can't prove `t < 5`
