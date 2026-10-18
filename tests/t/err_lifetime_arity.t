#[derive(Discard)]
struct View<'a> { r: &'a int }

fn f(v: View<'static, 'static>) {}

fn main() {}
// error: `View` takes 1 lifetime argument but 2 were supplied
