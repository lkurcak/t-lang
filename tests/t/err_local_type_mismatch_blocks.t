// Two in different blocks of the same function.
fn main() {
    let p = {
        #[derive(Discard)]
        struct P { x: i64 }
        P { x: 1 }
    };
    let q = {
        #[derive(Discard)]
        struct P { x: i64 }
        P { x: 2 }
    };
    let r = if true { p } else { q };
    print(r.x);
}
// error: `if` and `else` have different types: `P` and `P`; these are two different types called `P`, declared in different blocks in `main`
