// `return`, `break`, `continue` and `loop`. Each of the first three is an
// expression of type `!`, which fits wherever a value is wanted, since it
// never gives one.

#[derive(Discard)]
enum Option<T> { Some(T), None }

#[derive(Discard)]
enum State { Ok, Unauthorized }

fn login(user_exists: bool) -> State {
    if !user_exists { return State::Unauthorized; }
    State::Ok
}

fn show(s: State) -> i64 {
    match s {
        State::Ok => 1,
        State::Unauthorized => 0,
    }
}

// The `None` arm is a `!`, which fits as the `i64` the other arm is.
fn or_minus_one(o: Option<i64>) -> i64 with panic {
    let n = match o {
        Option::Some(n) => n,
        Option::None => return -1,
    };
    n * 10
}

// A `loop`'s value is what its `break` gives it.
fn first_square_above(n: i64) -> i64 with panic {
    let mut i = 0;
    loop {
        if i * i > n { break i * i; }
        i += 1;
    }
}

// `continue` skips the rest of the body, `break` leaves the loop.
fn sum_odd_below(n: i64) -> i64 with panic {
    let mut sum = 0;
    let mut i = 0;
    while true {
        i += 1;
        if i >= n { break; }
        if i % 2 == 0 { continue; }
        sum += i;
    }
    sum
}

// A `break` leaves the innermost loop only.
fn pairs(n: i64) -> i64 with panic {
    let mut count = 0;
    let mut i = 0;
    while i < n {
        let mut j = 0;
        loop {
            if j == i { break; }
            count += 1;
            j += 1;
        }
        i += 1;
    }
    count
}

// A body that ends in a `return` is a `!`, and fits the result type.
fn sign(n: i64) -> i64 {
    if n < 0 { return -1; }
    if n == 0 { return 0; }
    return 1;
}

// `return a + b` returns `a + b`: what follows a `return` runs as far right
// as an expression can.
fn add(a: i64, b: i64) -> i64 with panic {
    return a + b;
}

// `return` with nothing after it returns `()`.
fn keep_positive(n: i64, kept: &mut i64) {
    if n <= 0 { return; }
    *kept = n;
}

fn apply(f: fn(i64) -> i64, x: i64) -> i64 { f(x) }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut stdout = os.stdout;
    stdout.line(&show(login(true)))?;                       // expect: 1
    stdout.line(&show(login(false)))?;                      // expect: 0
    stdout.line(&or_minus_one(Option::Some(4)))?;           // expect: 40
    stdout.line(&or_minus_one(Option::None))?;              // expect: -1
    stdout.line(&first_square_above(10))?;                  // expect: 16
    stdout.line(&sum_odd_below(10))?;                       // expect: 25
    stdout.line(&pairs(4))?;                                // expect: 6
    stdout.line(&(sign(-5) + sign(0) * 10 + sign(7) * 100))?; // expect: 99
    stdout.line(&add(2, 3))?;                               // expect: 5
    let mut kept = 0;
    keep_positive(-1, &mut kept);
    stdout.line(&kept)?;                               // expect: 0
    keep_positive(8, &mut kept);
    stdout.line(&kept)?;                               // expect: 8

    // A `return` in a closure returns from the closure, whose result type
    // is worked out from it.
    let clamp = |x| {
        if x > 10 { return 10; }
        x
    };
    stdout.line(&(apply(clamp, 50) + apply(clamp, 3)))?;      // expect: 13

    // `if` and `else` can be a `!` and an `i64`.
    let mut i: i64 = 0;
    let found = loop {
        let v = if i < 3 { i * 2 } else { break -1; };
        if v == 4 { break v; }
        i += 1;
    };
    stdout.line(&found)?;                                   // expect: 4
    Result::Ok(())
}
