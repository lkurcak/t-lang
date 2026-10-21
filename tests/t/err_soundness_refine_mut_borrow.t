// Through `&mut d`, `zero` changes `d`.
fn zero(r: &mut int) { *r = 0; }

fn f() -> int {
    let mut d = 1;
    zero(&mut d);
    10 / d
}

fn main() with panic {
    print(f());
}
// error: `/` may panic, but `f` doesn't say `with panic`
