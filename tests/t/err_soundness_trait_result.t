// An impl's method gives what its trait's says, whether or not it says it
// again: generic code counts on it (docs/86-trait-contracts.md).
trait Small {
    fn get(&self) -> { v: usize | v < 10 };
}

#[derive(Discard)]
struct S {}

impl Small for S {
    fn get(&self) -> usize { 10 }
}

fn index<T: Small>(t: &T) -> u8 {
    let a = [0u8; 10];
    a[t.get()]
}

fn main() { let _ = index(&S {}); }
// error: can't prove `v < 10` for the result of `get`
