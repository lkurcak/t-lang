// A program's own items come before the prelude's: this `Option` is not
// `core`'s, which is still there by its path.

#[derive(Discard)]
enum Option { Some(int), None }

fn mine(n: int) -> Option { if n > 0 { Option::Some(n) } else { Option::None } }

fn theirs(n: int) -> core::option::Option<int> { core::option::Option::Some(n) }

mod inner {
    // A module sees the prelude, not the crate root's `Option`.
    pub fn wrap(n: int) -> Option<int> { Option::Some(n) }
}

// Nor can a program's own `Iterator`, which isn't what `for` uses.
trait Iterator {
    fn next(&mut self) -> int;
}

fn main() with panic {
    let mut sum = 0;
    for i in 0..4 {
        sum = sum + i;
    }
    print(sum); // expect: 6
    match mine(1) {
        Option::Some(n) => print(n), // expect: 1
        Option::None => {}
    }
    print(theirs(2).unwrap_or(0));      // expect: 2
    print(inner::wrap(3).unwrap_or(0)); // expect: 3
}
