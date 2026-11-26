// A const is a pattern where a literal of its type could be: of an
// integer type, `bool`, `char` or `&str`.
#[derive(Copy, Discard)]
struct P { x: u8 }

const ORIGIN: P = P { x: 0 };

fn f(p: P) -> u8 {
    match p {
        ORIGIN => 0,
        _ => 1,
    }
}

fn main() {
    let _ = f(P { x: 1 });
}
// error: `ORIGIN` is a const of type `P`: a pattern can name a const of an integer type, `bool`, `char` or `&str`
