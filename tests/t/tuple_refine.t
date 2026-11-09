// A tuple's fields are functions of it, as a struct's are: a predicate
// names them as `p.0`, and what's known of the values a tuple is made
// of is known of its fields, through patterns too.
fn ordered(p: { p: (usize, usize) | p.0 < p.1 }) -> usize { p.1 - p.0 }

fn split(n: { n: u8 | n >= 2 }) -> { p: (u8, u8) | p.0 + p.1 == n && p.0 >= 1 } {
    (1, n - 1)
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: [u8; 4] = [10, 20, 30, 40];
    out.line(&ordered((1, 3)))?;    // expect: 2
    let i: usize = 2;
    let p = (i, i + 1);
    out.line(&a[p.1])?;             // expect: 40
    let (j, k) = p;
    out.line(&a[k - j])?;           // expect: 20
    let s = split(5);
    let rest: { r: u8 | r <= 4 } = s.1;
    out.line(&rest)?;               // expect: 4
    match a.as_slice().split_at(1) {
        Some((head, tail)) => {
            out.line(&head[0])?;    // expect: 10
            out.line(&tail[2])?;    // expect: 40
        },
        None => {},
    }
    if let Some(halves) = "hello".split_at(2) {
        out.line(&halves.0)?;       // expect: he
        out.line(&halves.1.len())?; // expect: 3
    }
    Ok(())
}
