// An exported function's refinement names only exported fields.
#[derive(Copy, Discard)]
export struct P { pub x: u8 }

export fn x_of(p: P) -> { v: u8 | v == p.x } { p.x }

fn main() {
    let _ = x_of(P { x: 1 });
}
// error: `x_of` is exported, but its refinement names field `x` of `P`, which isn't exported
