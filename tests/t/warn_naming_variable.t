// Parameters and variables are named in snake_case: those of functions,
// `let`s, closures and patterns. A variable bound in every alternative of
// an or-pattern is warned about once. A leading `_` doesn't count, and
// digits are fine.
#[derive(Discard)]
enum Shape { Circle(i64), Square(i64), Empty }

fn side(s: Shape, Default: i64) -> i64 {
    match s {
        Shape::Circle(sideLen) | Shape::Square(sideLen) => sideLen,
        Shape::Empty => Default,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let firstTry = side(Shape::Circle(2), 0);
    let x1 = side(Shape::Square(3), 0);
    let _ignoredValue = 4i64;
    let addTo = |baseValue: i64| baseValue + firstTry;
    out.line(&(addTo(x1) + side(Shape::Empty, 5)))?;
    Result::Ok(())
}
// warning: variable `Default` should have a snake case name: `default`
// warning: variable `sideLen` should have a snake case name: `side_len`
// warning: variable `firstTry` should have a snake case name: `first_try`
// warning: variable `_ignoredValue` should have a snake case name: `_ignored_value`
// warning: variable `addTo` should have a snake case name: `add_to`
// warning: variable `baseValue` should have a snake case name: `base_value`
// expect: 10
