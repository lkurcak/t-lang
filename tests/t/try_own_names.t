// A program's own `Try`, `ControlFlow`, `Result` and `Option` don't change
// what `?` means: it names the core library's.
#[derive(Discard)]
enum ControlFlow { Continue, Break }

trait Try { fn branch(self) -> i64; }

#[derive(Discard)]
enum Option { Some, None }

fn get(x: i64) -> core::option::Option<i64> {
    if x > 0 { core::option::Option::Some(x) } else { core::option::Option::None }
}

fn twice(x: i64) -> core::option::Option<i64> with panic { core::option::Option::Some(get(x)? * 2) }

fn main() with panic {
    print(twice(5).unwrap_or(0)); // expect: 10
    let _ = ControlFlow::Continue;
    let _ = ControlFlow::Break;
    let _ = Option::Some;
    let _ = Option::None;
}
