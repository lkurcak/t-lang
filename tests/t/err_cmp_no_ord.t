#[derive(Discard, Eq)]
struct P { x: i64 }

fn main() {
    let a = P { x: 1 };
    let b = P { x: 2 };
    let _less = a < b;
}
// error: cannot compare `P` with `<`: it doesn't implement `Ord`
