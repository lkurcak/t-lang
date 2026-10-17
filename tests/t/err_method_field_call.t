struct F { f: fn() -> int }
fn one() -> int { 1 }
fn main() {
    let s = F { f: one };
    print(s.f());
}
// error: `F` has no method `f`, but it has a field `f`, which holds a function; to call that, put the field in parentheses
