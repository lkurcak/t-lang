// Labels: `'a:` before a `loop` or `while` lets a `break` or `continue` in a
// loop inside it say it's for that loop.

fn consume(b: Box<int>) -> int { *b }

// The first pair of numbers below `n` that multiply to `target`: a `break`
// with a label leaves both loops at once, with a value.
fn find(n: int, target: int) -> int with panic {
    let mut i = 1;
    'outer: loop {
        let mut j = 1;
        while j < n {
            if i * j == target { break 'outer i * 10 + j; }
            j += 1;
        }
        i += 1;
        if i == n { break -1; }
    }
}

// `continue 'rows` goes on to the next row from inside the inner loop.
fn count_before_zero(rows: int, cols: int) -> int {
    let mut count = 0;
    let mut r = 0;
    'rows: while r < rows {
        r += 1;
        let mut c = 0;
        while c < cols {
            c += 1;
            if c == r { continue 'rows; }
            count += 1;
        }
    }
    count
}

// The inner `loop` never ends by a `break` of its own, so it's a `!`, and
// the code after it is unreachable: nothing gets past it but `break 'a`.
fn inner_never() -> int {
    'a: loop {
        loop {
            break 'a 7;
        }
    }
}

fn main() with {panic, alloc} {
    print(find(10, 12));                    // expect: 26
    print(find(3, 100));                    // expect: -1
    print(count_before_zero(3, 3));         // expect: 3
    print(inner_never());                   // expect: 7

    // Boxes in both loops' scopes are freed by the labelled `break`.
    let mut sum = 0;
    'outer: loop {
        let a = Box::new(1);
        loop {
            let b = Box::new(2);
            sum += *a + *b;
            if sum > 5 { break 'outer; }
        }
    }
    print(sum);                             // expect: 6

    // Moved, then out of both loops for good.
    let b = Box::new(4);
    'done: loop {
        loop {
            print(consume(b));              // expect: 4
            break 'done;
        }
    }

    // A loop in a closure can have the same label as one outside it: a
    // `break` can't get from one to the other anyway.
    'a: loop {
        let f = |n: int| {
            let mut i = 0;
            'a: loop {
                if i == n { break 'a i; }
                i += 1;
            }
        };
        print(f(3));                        // expect: 3
        break 'a;
    }
}
