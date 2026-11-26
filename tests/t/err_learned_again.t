// What a branch learns of a value is learned again by the other, which
// can't use the first's (lemma K1 of docs/61-proof-search.md): `u`, a copy
// of a variable declared with a refinement, has it in either branch.
#[derive(Copy, Discard)]
struct Q { x: u32, y: u32 }

fn need(q: &{ v: Q | v.x <= v.y }) with {} {
    let d: u32 = q.y - q.x;
    let _ = d;
}

fn main(os: std::Os) {
    let p = (Q { x: 0, y: 10 }, 0u32);
    if os.args.len() > 0 {
        let a: ({ v: Q | v.x <= v.y }, u32) = p;
        let _ = a;
    } else {
        let a: ({ v: Q | v.x <= v.y }, u32) = p;
        let mut u = a;
        let mut d = || u.0.x = 50; // error: can't prove `v.x <= v.y` for `u.0`, once `u.0.x` is written
        d();
        need(&u.0);
    }
}
