// `+=` overflows as `+` does, so it may panic too. (Step 7 missed it.)
fn bump(x: int) -> int {
    let mut y = x;
    y += 1;
    y
}

fn main() with panic {
    print(bump(9223372036854775807));
}
// error: `+=` may panic, but `bump` doesn't say `with panic`
