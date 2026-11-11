// A closure captures `o`, so what's known of it is forgotten (it's
// unstable) and each read of it is a new value. Within an arm, what the
// pattern binds is still of the value matched: `n` isn't 3, as the arm
// before took `Some(3)`.
fn not3(o: Option<u8>) -> { v: u8 | v != 3 } {
    let mut o = o;
    let mut set = |v: u8| {
        o = Some(v);
    };
    set(4);
    match o {
        Some(3) => 0,
        Some(n) => n,
        None => 0,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&not3(Some(5)))?; // expect: 4
    Ok(())
}
