// Getting what's in a `Result` whose error type is `!` needs nothing for an
// `Err`, which it can't be: `unwrap` and `expect` say they may panic, and
// `unwrap_or` and `unwrap_or_else` make a default or pass a function for
// nothing. `into_ok` is the fix. So is it of a sink's result, once the sink
// is known, as a `String` is.
use alloc::string::String;
use core::fmt::Write;

fn sure(x: i64) -> Result<i64, !> { Result::Ok(x) }

fn main() with {alloc, panic} {
    print(sure(1).unwrap());              // expect: 1
    print(sure(2).expect("can't fail"));  // expect: 2
    print(sure(3).unwrap_or(0));          // expect: 3
    print(sure(4).unwrap_or_else(|e| e)); // expect: 4
    let mut s = String::new();
    s.write_str("ok").unwrap();
    print_str(s.as_str()); // expect: ok
    s.free();
}
// warning: `unwrap` of a `Result` whose error type is `!`, which can't be an `Err`: it says it may panic, though it can't
// warning: `expect` of a `Result` whose error type is `!`, which can't be an `Err`: it says it may panic, though it can't
// warning: `unwrap_or` of a `Result` whose error type is `!`, which can't be an `Err`: the default is made for nothing
// warning: `unwrap_or_else` of a `Result` whose error type is `!`, which can't be an `Err`: the function is never called
// warning: `unwrap` of a `Result` whose error type is `!`, which can't be an `Err`: it says it may panic, though it can't
