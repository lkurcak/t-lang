trait Show { fn show(&self); }
enum Option<T> { Some(T), None }
impl<T: Show> Show for Option<T> { fn show(&self) {} }
fn main() { Option::Some(true).show(); }
// error: `Option<bool>` doesn't implement `Show`, which method `show` is of; that needs `bool: Show`
