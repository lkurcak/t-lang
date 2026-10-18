trait Show { fn show(&self); }
#[derive(Discard)]
struct P {}
impl<T> Show for P { fn show(&self) {} }
fn main() {}
// error: type parameter `T` isn't in `P`, so nothing could say what it is where the impl is used
