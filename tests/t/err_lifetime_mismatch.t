fn second<'a, 'b>(_x: &'a i64, y: &'b i64) -> &'a i64 { y }

fn main() {}
// error: lifetime may not live long enough: this needs `'b` to outlive `'a`
