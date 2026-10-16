// A part of a variable with no value can't be given one: the whole
// hasn't got one to be part of.
struct P { x: int, y: int }

fn main() {
    let mut p: P;
    p.x = 1;
    p.y = 2;
    print(p.x);
}
// error: cannot assign to `p.x`, part of `p`, which has no value here
