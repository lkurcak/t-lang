#[derive(Discard)]
enum Op { Apply(fn(int) -> int), Skip }
fn main() {
    let n = 5;
    let op = Op::Apply(|x| x + n);
    match op {
        Op::Apply(f) => print(f(1)),
        Op::Skip => (),
    }
}
// error: cannot store a closure that captures `n` in a struct or enum
