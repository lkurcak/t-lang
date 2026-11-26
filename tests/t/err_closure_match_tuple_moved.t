// The closure owns `t.0` now (it captures the place it moves out of):
// `t.0` can't be used after.
#[derive(Discard)]
struct D { n: u8 }

fn take(d: D) -> u8 {
    let D { n } = d;
    n
}

fn main() {
    let t = (D { n: 4 }, 2u8);
    let c = || match t {
        (a, _) => take(a),
    };
    let _ = c();
    let _ = take(t.0);
}
// error: use of moved value `t`
