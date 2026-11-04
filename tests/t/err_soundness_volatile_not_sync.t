// A register shared by two threads needs more than volatile accesses: a
// `&Volatile` can't be handed to another one.
use core::volatile::Volatile;

fn send<T: Send + Discard>(_t: T) {}

fn f(r: &'static Volatile<u32>) {
    send(r);
// error: `Volatile<u32>: Sync`
}

fn main() {}
