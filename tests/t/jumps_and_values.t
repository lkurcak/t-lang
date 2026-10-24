// How `return`, `break` and `continue` meet variables given their values
// later, moves, borrows and closures. The checks follow the paths control
// can take: a `return` or `break` ends one.

use alloc::boxed::Box;

fn consume(b: Box<i64>) -> i64 { *b }

// `x` has a value on every path that gets to the `line`: the other returns.
fn deferred_return(out: &mut std::io::Stdout, c: bool) -> Result<(), std::io::Error> {
    let x: i64;
    if c { x = 1; } else { return Result::Ok(()); }
    out.line(&x)
}

// The only way out of the `loop` is the `break`, after `x` gets its value.
fn deferred_loop() -> i64 with panic {
    let x;
    let mut i = 0;
    loop {
        i += 1;
        if i == 3 {
            x = i * 10;
            break;
        }
    }
    x
}

// A `break` in a `while`'s condition leaves the loop around the `while`:
// the condition isn't part of the `while`'s body.
fn break_in_condition() -> i64 with panic {
    let mut n = 0;
    loop {
        while if n > 4 { break; } else { true } {
            n += 1;
            if n % 2 == 0 { break; }
        }
        n += 10;
    }
    n
}

// Borrowed on one path, which returns; changed on the other.
fn borrow_or_change(c: bool) -> i64 {
    let mut x = 1;
    let r = &mut x;
    if c { return *r; }
    x = 2;
    x
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {panic, alloc} {
    let mut out = os.stdout;
    deferred_return(&mut out, true)?;       // expect: 1
    deferred_return(&mut out, false)?;
    out.line(&deferred_loop())?;                 // expect: 30
    out.line(&break_in_condition())?;            // expect: 12

    out.line(&borrow_or_change(true))?;          // expect: 1
    out.line(&borrow_or_change(false))?;         // expect: 2

    // Moved, then out of the loop for good.
    let b = Box::new(7);
    let mut i: i64 = 0;
    while i < 10 {
        i += 1;
        if i == 2 {
            out.line(&consume(b))?;              // expect: 7
            break;
        }
    }

    // A closure that captures can leave a loop, as long as it isn't
    // returned or stored.
    let n: i64 = 5;
    let f = loop {
        break |x| x + n;
    };
    out.line(&f(1))?;                            // expect: 6
    Result::Ok(())
}
