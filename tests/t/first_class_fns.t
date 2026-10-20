// Functions are values: they can be passed, returned and stored in variables.

fn inc(x: int) -> int with panic { x + 1 }
fn double(x: int) -> int with panic { x * 2 }

fn apply(f: fn(int) -> int with panic, x: int) -> int with panic { f(x) }
fn twice(f: fn(int) -> int with panic, x: int) -> int with panic { f(f(x)) }

fn pick(first: bool) -> fn(int) -> int with panic {
    if first { inc } else { double }
}

fn main() with panic {
    print(apply(inc, 1));           // expect: 2
    print(twice(double, 3));        // expect: 12
    print(pick(false)(10));         // expect: 20
    let p: fn(int) -> () = print;   // even builtins are values
    p(7);                           // expect: 7
}
