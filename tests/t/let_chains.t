// `if` and `while` take a chain: conditions joined by `&&`, each a `let`
// or a `bool`. What a `let` binds is there in the conditions after it and
// in the block, never in the `else`; the `else` runs if any condition
// doesn't hold, and the conditions after that one aren't worked out.
fn first_big(o: Option<u8>) -> u8 {
    if let Some(a) = o && a > 3 { a } else { 0 }
}

fn both(a: Option<u8>, b: Option<u8>) -> u8 {
    if let Some(x) = a && let Some(y) = b && x < 100 && y < 100 {
        x + y
    } else if let Some(x) = a && x < 200 {
        x + 1
    } else {
        7
    }
}

// A `bool` can come first.
fn leading(c: bool, o: Option<u8>) -> u8 {
    if c && let Some(a) = o { a } else { 9 }
}

fn noisy(out: &mut std::io::Stdout, n: u8) -> Option<u8> {
    let _ = out.line(&n);
    Some(n)
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&first_big(Some(5)))?; // expect: 5
    out.line(&first_big(Some(2)))?; // expect: 0
    out.line(&first_big(None))?; // expect: 0
    out.line(&both(Some(1), Some(2)))?; // expect: 3
    out.line(&both(Some(1), None))?; // expect: 2
    out.line(&both(None, None))?; // expect: 7
    out.line(&leading(true, Some(4)))?; // expect: 4
    out.line(&leading(false, Some(4)))?; // expect: 9
    // No `else`: a `()`. The second scrutinee isn't worked out when the
    // first `let` fails.
    if let Some(_) = Option::<u8>::None && let Some(n) = noisy(&mut out, 1) {
        out.line(&n)?;
    }
    if let Some(m) = noisy(&mut out, 2) && let Some(n) = noisy(&mut out, m + 1) { // expect: 2
        out.line(&(m + n))?; // expect: 3
    } // expect: 5
    // What a `let` binds holds where the block runs: `i < 4`.
    let a = [10u8, 11, 12, 13];
    let at = Some(2usize);
    if let Some(i) = at && i < 4 {
        out.line(&a[i])?; // expect: 12
    }
    // `while` too: it stops at the first condition that doesn't hold.
    let mut it = 0u8..10;
    let mut sum = 0u8;
    while let Some(i) = it.next() && i < 5 {
        if i == 1 {
            continue;
        }
        sum = sum + i;
    }
    out.line(&sum)?; // expect: 9
    let mut k = 0u8;
    'outer: while k < 3 && let Some(j) = Some(k) {
        k = k + 1;
        if j == 1 {
            break 'outer;
        }
    }
    out.line(&k)?; // expect: 2
    Ok(())
}
