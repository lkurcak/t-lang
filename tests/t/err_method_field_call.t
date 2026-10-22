#[derive(Discard)]
struct F { f: fn() -> i64 }
fn one() -> i64 { 1 }
fn main() {
    let s = F { f: one };
    print(s.f());
}
// error: `F` has no method `f`, but it has a field `f`, which holds a function; to call that, put the field in parentheses
