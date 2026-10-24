// Variables nothing reads: `let`s, parameters and pattern bindings. The fix
// renames each to start with `_`.
#[derive(Discard)]
struct P { x: i64, y: i64 }
#[derive(Discard)]
enum Opt { None, Some(i64) }

fn first(a: i64, b: i64) -> i64 {
    a
}

fn get(o: Opt) -> i64 {
    match o {
        Opt::Some(n) => 1,
        Opt::None => 0,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let unused: i64 = 1;
    // Shorthand: the fix writes `P { x, y: _y }`, not `P { x, _y }`.
    let P { x, y } = P { x: 2, y: 3 };
    out.line(&x)?;
    // A shadowed variable that was never read.
    let z: i64 = 4;
    let z: i64 = 5;
    out.line(&z)?;
    out.line(&first(6, 7))?;
    out.line(&get(Opt::Some(8)))?;
    out.line(&get(Opt::None))?;
    Result::Ok(())
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
