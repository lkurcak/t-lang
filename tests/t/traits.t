// Traits: a set of methods that types can implement, each in an `impl`.

trait Show {
    fn show(&self);

    /// A default, which an impl may leave out.
    fn show_twice(&self) with panic {
        self.show();
        self.show();
    }
}

#[derive(Discard, Copy)]
struct Point { x: i64, y: i64 }

impl Show for Point {
    fn show(&self) {
        print(self.x);
        print(self.y);
    }
}

// Any type can implement a trait, `i64` too, and override a default.
impl Show for i64 {
    fn show(&self) { print(*self) }

    fn show_twice(&self) with panic { print(*self * 11) }
}

#[derive(Discard)]
enum Option<T> { Some(T), None }

// For every `T` that's `Show`: a generic impl.
impl<T: Show> Show for Option<T> {
    fn show(&self) {
        match self {
            Option::Some(x) => x.show(),
            Option::None => print(0i64),
        }
    }
}

// A bound: `T` must be `Show`, so values of it have `show`.
fn show_all<T: Show + Discard>(a: T, b: T) {
    a.show();
    b.show();
}

fn main() with panic {
    let p = Point { x: 1, y: 2 };
    p.show();                   // expect: 1
                                // expect: 2
    7i64.show_twice();             // expect: 77
    Option::Some(3i64).show();     // expect: 3
    Option::<i64>::None.show(); // expect: 0
    show_all(4i64, 5);             // expect: 4
                                // expect: 5
    // Paths: the trait's method, or a type's.
    Show::show(&6i64);             // expect: 6
    Point::show_twice(&p);      // expect: 1
                                // expect: 2
                                // expect: 1
                                // expect: 2
    let f = Option::<Point>::show;
    f(&Option::Some(p));        // expect: 1
                                // expect: 2
}
