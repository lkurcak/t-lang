// Through a `&mut T` a function can give back what was behind it, which
// isn't an argument's value: a `T` behind a reference isn't a way to one.
fn put<T>(slot: &mut T, v: T) -> T { core::mem::replace(slot, v) }

fn at(a: [u8; 4], i: { v: usize | v < 4 }) -> u8 {
    let mut s: usize = 9;
    let j = put(&mut s, i);
    a[j]
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
