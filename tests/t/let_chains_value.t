// A chain with an `else` is a value: what the block gives is known where
// the chain holds, and the refinement wanted is proved of each branch.
fn small(o: Option<u8>) -> { v: u8 | v < 10 } {
    if let Some(a) = o && a < 10 { a } else { 0 }
}

// A block that leaves gives nothing to the value.
fn or_return(o: Option<u8>) -> u8 {
    let n = if let Some(a) = o && a > 0 { return a } else { 3 };
    n + 1
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&small(Some(4)))?; // expect: 4
    out.line(&small(Some(40)))?; // expect: 0
    out.line(&or_return(Some(5)))?; // expect: 5
    out.line(&or_return(None))?; // expect: 4
    Ok(())
}
