// An impl's method that says something of its result still gives what
// its trait's says: what it says adds to that (docs/86-trait-contracts.md).
trait Small {
    fn get(&self) -> { v: usize | v < 10 };
}

#[derive(Discard)]
struct S {}

impl Small for S {
    fn get(&self) -> { v: usize | v < 100 } { 50 }
}

fn index<T: Small>(t: &T) -> u8 {
    let a = [0u8; 10];
    a[t.get()]
}

fn main() { let _ = index(&S {}); }
// error: can't prove `v < 10` for the result of `get`
