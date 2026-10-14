// Reading a field needs the type now: which struct is it?
struct P { x: int }
fn main() {
    let get = |p| p.x;
    print(get(P { x: 1 }));
}
// error: the type of this must be known to read its field `x`
