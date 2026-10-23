fn f(x: Option<i64>) -> Option<i64> { ?x }

fn main() { let _ = f(Option::None); }
// error: expected an expression, found `?`
