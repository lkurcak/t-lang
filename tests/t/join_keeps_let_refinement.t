// A `let mut` with a refinement, changed in a loop that may `break`, keeps
// its refinement after the loop: the loop's end, which never read it, and
// the `break`, which did, meet, and every value it had has the refinement,
// as each write proved it (lemma J3 of docs/61-proof-search.md). Found
// writing step 102's slow paths.
fn need(g: { g: u32 | g <= 5 }) -> u32 with {} { g }

fn count_down(x: u32) -> u32 with {} {
    let mut g: { g: u32 | g <= 5 } = 5;
    for _ in 0..5u32 {
        if g == 0 || x == 3 {
            break;
        }
        g = g - 1;
    }
    need(g + 0)
}

fn halve(x: u32) -> u32 with {} {
    let mut g: { g: u32 | g <= 5 } = 5;
    while x > 0 {
        if g == 0 {
            break;
        }
        g = g / 2;
    }
    need(g)
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&count_down(1))?; // expect: 0
    out.line(&count_down(3))?; // expect: 5
    out.line(&halve(0))?;      // expect: 5
    Ok(())
}
