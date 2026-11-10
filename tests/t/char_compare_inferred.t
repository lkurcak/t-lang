// A comparison whose operands' type is decided only after it's checked,
// here by the call of the closure: of `char`s it's the machine's
// comparison of their numbers all the same, which the checker reasons
// about, so `d > c` follows from `c < d`.
fn yes(_b: { b: bool | b }) {}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let lt = |c, d| if c < d {
        yes(d > c);
        true
    } else {
        false
    };
    out.line(&lt('a', 'b'))?; // expect: true
    out.line(&lt('é', 'z'))?; // expect: false
    Ok(())
}
