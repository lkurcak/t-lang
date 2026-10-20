// `Result` comes from `core`, through the prelude.

fn div(a: int, b: int) -> Result<int, bool> with panic {
    if b == 0 { Result::Err(false) } else { Result::Ok(a / b) }
}

fn main() with panic {
    let ok = div(6, 3);
    let err = div(1, 0);
    print_bool(ok.is_ok());   // expect: true
    print_bool(err.is_err()); // expect: true
    print(ok.unwrap_or(0));   // expect: 2
    print(err.unwrap_or(-1)); // expect: -1
    print(ok.map(|x| x + 1).unwrap_or(0)); // expect: 3
    print_bool(err.map_err(|e| !e).is_err()); // expect: true
    print(ok.and_then(|x| div(x, 2)).unwrap_or(0)); // expect: 1
    print(ok.and_then(|x| div(x, 0)).unwrap_or(-1)); // expect: -1
    print(err.or_else(|_e| div(9, 3)).unwrap_or(0)); // expect: 3
    print(ok.or_else(|_e| div(9, 3)).unwrap_or(0));  // expect: 2
    print_bool(err.or_else(|e| div(1, if e { 1 } else { 0 })).is_err()); // expect: true
    print(err.unwrap_or_else(|e| if e { 1 } else { 0 })); // expect: 0
    print(ok.unwrap_or_else(|_e| 0)); // expect: 2
    print(ok.ok().unwrap_or(0));  // expect: 2
    print_bool(ok.err().is_none()); // expect: true
    match err.as_ref() {
        Result::Ok(n) => print(*n),
        Result::Err(e) => print_bool(*e), // expect: false
    }
}
