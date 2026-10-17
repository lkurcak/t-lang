// How `return`, `break` and `continue` meet variables given their values
// later, moves, borrows and closures. The checks follow the paths control
// can take: a `return` or `break` ends one.

fn consume(b: Box<int>) -> int { *b }

// `x` has a value on every path that gets to the `print`: the other returns.
fn deferred_return(c: bool) {
    let x;
    if c { x = 1; } else { return; }
    print(x);
}

// The only way out of the `loop` is the `break`, after `x` gets its value.
fn deferred_loop() -> int {
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
fn break_in_condition() -> int {
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
fn borrow_or_change(c: bool) -> int {
    let mut x = 1;
    let r = &mut x;
    if c { return *r; }
    x = 2;
    x
}

fn main() {
    deferred_return(true);                  // expect: 1
    deferred_return(false);
    print(deferred_loop());                 // expect: 30
    print(break_in_condition());            // expect: 12

    print(borrow_or_change(true));          // expect: 1
    print(borrow_or_change(false));         // expect: 2

    // Moved, then out of the loop for good.
    let b = Box::new(7);
    let mut i = 0;
    while i < 10 {
        i += 1;
        if i == 2 {
            print(consume(b));              // expect: 7
            break;
        }
    }

    // A closure that captures can leave a loop, as long as it isn't
    // returned or stored.
    let n = 5;
    let f = loop {
        break |x| x + n;
    };
    print(f(1));                            // expect: 6
}
