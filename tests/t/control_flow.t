// `return`, `break`, `continue` and `loop`. Each of the first three is an
// expression of type `!`, which fits wherever a value is wanted, since it
// never gives one.

enum Option<T> { Some(T), None }

enum State { Ok, Unauthorized }

fn login(user_exists: bool) -> State {
    if !user_exists { return State::Unauthorized; }
    State::Ok
}

fn show(s: State) -> int {
    match s {
        State::Ok => 1,
        State::Unauthorized => 0,
    }
}

// The `None` arm is a `!`, which fits as the `int` the other arm is.
fn or_minus_one(o: Option<int>) -> int {
    let n = match o {
        Option::Some(n) => n,
        Option::None => return -1,
    };
    n * 10
}

// A `loop`'s value is what its `break` gives it.
fn first_square_above(n: int) -> int {
    let mut i = 0;
    loop {
        if i * i > n { break i * i; }
        i += 1;
    }
}

// `continue` skips the rest of the body, `break` leaves the loop.
fn sum_odd_below(n: int) -> int {
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
fn pairs(n: int) -> int {
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
fn sign(n: int) -> int {
    if n < 0 { return -1; }
    if n == 0 { return 0; }
    return 1;
}

// `return a + b` returns `a + b`: what follows a `return` runs as far right
// as an expression can.
fn add(a: int, b: int) -> int {
    return a + b;
}

// `return` with nothing after it returns `()`.
fn print_positive(n: int) {
    if n <= 0 { return; }
    print(n);
}

fn apply(f: fn(int) -> int, x: int) -> int { f(x) }

fn main() {
    print(show(login(true)));                       // expect: 1
    print(show(login(false)));                      // expect: 0
    print(or_minus_one(Option::Some(4)));           // expect: 40
    print(or_minus_one(Option::None));              // expect: -1
    print(first_square_above(10));                  // expect: 16
    print(sum_odd_below(10));                       // expect: 25
    print(pairs(4));                                // expect: 6
    print(sign(-5) + sign(0) * 10 + sign(7) * 100); // expect: 99
    print(add(2, 3));                               // expect: 5
    print_positive(-1);
    print_positive(8);                              // expect: 8

    // A `return` in a closure returns from the closure, whose result type
    // is worked out from it.
    let clamp = |x| {
        if x > 10 { return 10; }
        x
    };
    print(apply(clamp, 50) + apply(clamp, 3));      // expect: 13

    // `if` and `else` can be a `!` and an `int`.
    let mut i = 0;
    let found = loop {
        let v = if i < 3 { i * 2 } else { break -1; };
        if v == 4 { break v; }
        i += 1;
    };
    print(found);                                   // expect: 4
}
