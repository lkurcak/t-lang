// `return`, `break` and `continue` leave scopes and statements early, and
// what's in them is freed on the way out, once: the interpreter checks for
// leaks and double frees. So is what an expression had worked out before a
// jump out of the middle of it.

#[derive(Discard)]
struct Two { a: Box<int>, b: Box<int> }

fn consume(b: Box<int>) -> int { *b }

fn pair(a: Box<int>, b: int) -> int { *a + b }

// Both boxes are in scope at the `return`.
fn early(c: bool) -> int {
    let x = Box::new(1);
    let y = Box::new(2);
    if c { return *x; }
    *x + *y
}

// The box is moved on the path that returns, and still there on the other.
fn moved_then_return(c: bool) -> int {
    let b = Box::new(3);
    if c {
        let n = consume(b);
        return n;
    }
    *b
}

// The first argument is made, then the second returns: the first is freed.
fn half_made(c: bool) -> int {
    pair(Box::new(10), if c { return 0; } else { 5 })
}

// Out of a struct literal: `a` is made, `b` never is.
fn half_struct(c: bool) -> int {
    let t = Two { a: Box::new(1), b: if c { return 7; } else { Box::new(2) } };
    *t.a + *t.b
}

fn main() {
    print(early(true));                 // expect: 1
    print(early(false));                // expect: 3
    print(moved_then_return(true));     // expect: 3
    print(moved_then_return(false));    // expect: 3
    print(half_made(true));             // expect: 0
    print(half_made(false));            // expect: 15
    print(half_struct(true));           // expect: 7
    print(half_struct(false));          // expect: 3

    // A new box each time round, freed by `continue` and `break` too.
    let mut i = 0;
    let mut sum = 0;
    loop {
        let b = Box::new(i);
        i += 1;
        if i % 2 == 0 { continue; }
        if i > 6 { break; }
        sum += *b;
    }
    print(sum);                         // expect: 6

    // The value a `break` gives a loop leaves it, and isn't freed.
    let kept = loop {
        let b = Box::new(42);
        break b;
    };
    print(*kept);                       // expect: 42

    // Moved, then the loop left before the next time round.
    let b = Box::new(5);
    loop {
        print(consume(b));              // expect: 5
        break;
    }
}
