// Function values can go anywhere: functions, closures that capture nothing,
// and fields of function types, which hold only those.
#[derive(Discard, Copy)]
struct Fs { f: fn(int) -> int with panic, n: int }

fn inc(x: int) -> int with panic { x + 1 }
fn first(fs: Fs) -> fn(int) -> int with panic { fs.f }
fn unpack(fs: Fs) -> fn(int) -> int with panic {
    let Fs { f, .. } = fs;
    f
}
fn same(fs: Fs) -> Fs { fs }
fn n(x: int) -> int { x }
fn make() -> Fs {
    let g = inc;
    Fs { f: g, n: 1 }
}
fn call<e, F: Fn(int) -> int with e + Copy>(f: F) -> int with e {
    // A closure made from a parameter.
    let twice = |x| f(f(x));
    twice(0)
}

fn main() with panic {
    let fs = same(make());
    print(first(fs)(1));                // expect: 2
    print(unpack(Fs { f: |x| x * 10, n: 0 })(2)); // expect: 20
    print(n(3) + fs.n);                 // expect: 4
    let k = 7;
    print(call(|x| x + k));             // expect: 14
}
