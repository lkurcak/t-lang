// Not through a function either.
const A: u8 = f();

fn f() -> u8 { A }
// error: the value of const `A` depends on itself

fn main() {
    let _ = A;
}
