fn second<'a, 'b>(_x: &'a int, y: &'b int) -> &'a int { y }

fn main() {}
// error: lifetime may not live long enough: this needs `'b` to outlive `'a`
