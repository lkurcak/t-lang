// A panic's message, numbers in it included, is written through the
// platform's `t_platform_write` on the freestanding target, as stderr's on
// the hosted one (docs/39-freestanding.md): the same either way.
fn narrow(n: i64) -> u8 with panic { n as u8 }

fn main() {
    let small = narrow(-200 + 400);
    let _ = narrow(-9223372036854775807 + small as i64);
    // error: `as` overflowed: -9223372036854775607 isn't a `u8`
}
