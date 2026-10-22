// Calling a trait's method at a bigger type is fine if the types don't keep
// growing: `W<i64>` shows an `Option<&i64>`, which shows a `&i64`, which
// shows an `i64`.

trait Show {
    fn show(&self);
}

#[derive(Discard)]
enum Option<T> { Some(T), None }

impl<T: Show> Show for Option<T> {
    fn show(&self) {
        match self {
            Option::Some(x) => x.show(),
            Option::None => print(0i64),
        }
    }
}

impl<'a, T: Show> Show for &'a T {
    fn show(&self) { (**self).show() }
}

impl Show for i64 {
    fn show(&self) { print(*self) }
}

#[derive(Discard)]
struct W<T> { x: T }

impl<T: Show> Show for W<T> {
    fn show(&self) { Option::Some(&self.x).show() }
}

fn main() {
    W { x: W { x: 5i64 } }.show();         // expect: 5
    Option::<i64>::None.show();          // expect: 0
}
