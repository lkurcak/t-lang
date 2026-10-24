// No fix when the name it would get is a keyword, or would clash with
// another. The message only suggests a name that is one.
fn Match() -> i64 { 1 }
fn myFn() -> i64 { 2 }
fn my_fn() -> i64 { 3 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let True = Match();
    out.line(&(True + myFn() + my_fn()))?;
    Result::Ok(())
}
// warning: function `Match` should have a snake case name
// warning: function `myFn` should have a snake case name: `my_fn`
// warning: variable `True` should have a snake case name
// expect: 6
