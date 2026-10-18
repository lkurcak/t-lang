// `trait Sub: Discard` lets a bound `T: Sub` throw a `T` away, so only a
// `Discard` type can implement it.
struct Fd { n: int }

trait Sub: Discard {}

fn throw<T: Sub>(_x: T) {}

impl Sub for Fd {}

fn main() {
    throw(Fd { n: 1 });
}
// error: `Fd` doesn't implement `Discard`, which `Sub` needs, as it's `trait Sub: Discard`
