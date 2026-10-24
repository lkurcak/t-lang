// A derive names its traits as any path does: by a path, or by a name in
// scope, an import's too.
use core::cmp::Eq as Same;

#[derive(core::mem::Discard, core::mem::Copy, Same)]
struct P { x: i64 }

fn main() {
    let p = P { x: 1 };
    let q = p;
    print_bool(p == q); // expect: true
}
