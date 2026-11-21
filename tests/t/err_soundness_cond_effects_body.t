// A function saying `with panic if x > 10` may panic only when `x > 10`:
// `x + 250` overflows when `x > 5`, so it may panic when it says it won't
// (docs/90-conditional-effects.md).
fn bump(x: u8) -> u8 with panic if x > 10 { x + 250 }

fn main() with {} {
    let _v = bump(7);
}
// error: `+` may panic when `x > 10` doesn't hold, but `bump` says `with panic if x > 10`
