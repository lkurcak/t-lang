// Variables nothing reads: `let`s, parameters and pattern bindings. The fix
// renames each to start with `_`.
#[derive(Discard)]
struct P { x: int, y: int }
#[derive(Discard)]
enum Opt { None, Some(int) }

fn first(a: int, b: int) -> int {
    a
}

fn get(o: Opt) -> int {
    match o {
        Opt::Some(n) => 1,
        Opt::None => 0,
    }
}

fn main() {
    let unused = 1;
    // Shorthand: the fix writes `P { x, y: _y }`, not `P { x, _y }`.
    let P { x, y } = P { x: 2, y: 3 };
    print(x);
    // A shadowed variable that was never read.
    let z = 4;
    let z = 5;
    print(z);
    print(first(6, 7));
    print(get(Opt::Some(8)));
    print(get(Opt::None));
}
// warning: unused variable `b`
// warning: unused variable `n`
// warning: unused variable `unused`
// warning: unused variable `y`
// warning: unused variable `z`
// expect: 2
// expect: 5
// expect: 6
// expect: 1
// expect: 0
