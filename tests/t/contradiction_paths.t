// What's known on a path can't all be true: then nothing runs there, and
// every goal holds there, whichever variables it's of (docs/81). So a
// function that can't be called, as its argument's type has no values,
// can be called where nothing runs, and so can an index that would be out
// of bounds anywhere else.
fn absurd(_u: { u: () | false }) -> ! {
    loop {}
}

fn small(x: { x: u32 | x > 5 }, a: &[u32]) -> u32 with {} {
    if x < 3 {
        a[10]
    } else {
        x
    }
}

fn never(x: { x: u32 | x > 5 }) -> u32 with {} {
    if x < 3 {
        absurd(())
    }
    x
}

fn clamp(mut i: u32) -> { r: u32 | r >= 10 } with {} {
    while i < 10 {
        i += 1;
    }
    if i < 10 {
        absurd(())
    }
    i
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a = [1, 2, 3];
    out.line(&small(7, &a))?; // expect: 7
    out.line(&never(9))?; // expect: 9
    out.line(&clamp(4))?; // expect: 10
    Ok(())
}
