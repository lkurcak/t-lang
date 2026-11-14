// `{x:<6}`, `{x:>6}` and `{x:^6}` write `x` in at least 6 characters,
// with spaces after it, before it, or both, the odd one after; any
// character before the side is the fill instead. `?` and `x` after the
// width say how `x` is written, as without one.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let (name, ms) = ("pool", 1234u64);
    out.line(&f"[{name:<6}][{ms:>8}]")?; // expect: [pool  ][    1234]
    out.line(&f"[{name:^7}][{name:^8}]")?; // expect: [ pool  ][  pool  ]
    out.line(&f"[{name:*>6}][{ms:-<6}][{ms:_^7}]")?; // expect: [**pool][1234--][_1234__]
    // A value as wide as the width, or wider, is written as it is.
    out.line(&f"[{name:>4}][{name:<2}][{ms:^1}]")?; // expect: [pool][pool][1234]
    let p = [1i64, -3];
    out.line(&f"[{p:>9?}][{name:<7?}]")?; // expect: [  [1, -3]]["pool" ]
    let (h, b) = (255u8, true);
    out.line(&f"[{h:>4x}][{h:0<4x}][{b:>6}]")?; // expect: [  ff][ff00][  true]
    // The width counts characters, not bytes.
    let s = "é█";
    out.line(&f"[{s:>4}][{s:.^6}]") // expect: [  é█][..é█..]
}
