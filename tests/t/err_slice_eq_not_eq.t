// A slice compares only if its values do.
struct P { x: i64 }

fn same(a: &[P], b: &[P]) -> bool { a == b }

fn main() {}
// error: cannot compare `&[P]` with `==`: that needs `P: Eq`
