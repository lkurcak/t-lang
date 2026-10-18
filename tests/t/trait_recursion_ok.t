// Calling a trait's method at a bigger type is fine if the types don't keep
// growing: `W<int>` shows an `Option<&int>`, which shows a `&int`, which
// shows an `int`.

trait Show {
    fn show(&self);
}

#[derive(Discard)]
enum Option<T> { Some(T), None }

impl<T: Show> Show for Option<T> {
    fn show(&self) {
        match self {
            Option::Some(x) => x.show(),
            Option::None => print(0),
        }
    }
}

impl<'a, T: Show> Show for &'a T {
    fn show(&self) { (**self).show() }
}

impl Show for int {
    fn show(&self) { print(*self) }
}

#[derive(Discard)]
struct W<T> { x: T }

impl<T: Show> Show for W<T> {
    fn show(&self) { Option::Some(&self.x).show() }
}

fn main() {
    W { x: W { x: 5 } }.show();         // expect: 5
    Option::<int>::None.show();          // expect: 0
}
