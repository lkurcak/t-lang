// A pattern binding a tuple's element by `&mut`, where the tuple's type
// refines that element: what's written through the binding must have it,
// as through `let r = &mut t.1;` (step 62 checked what a `let` declared,
// not what the tuple's type says).
#![allow(dead_code)]
fn bad(r: &mut usize) { *r = 99 }

fn let_pattern(t: &mut (u8, { v: usize | v < 5 })) {
    let (_, b) = t;
    *b = 99;
}

fn match_arm(t: &mut (u8, { v: usize | v < 5 })) {
    match t {
        (_, b) => bad(b),
    }
}

fn main() {}
// error: can't prove `v < 5`
// error: can't prove `v < 5`
