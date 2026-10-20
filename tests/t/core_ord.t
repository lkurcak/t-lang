// `Ord` comes from `core`, through the prelude; `Ordering` needs a `use`.
// `int` and `bool` implement it, `false` before `true`.

use core::cmp::Ordering;

fn show(o: Ordering) -> int {
    match o {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

#[derive(Discard, Eq)]
struct Version { major: int, minor: int }

impl Ord for Version {
    fn cmp(&self, other: &Version) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => self.minor.cmp(&other.minor),
            o => o,
        }
    }
}

fn largest<T: Ord + Discard>(a: T, b: T, c: T) -> T {
    a.max(b).max(c)
}

fn main() {
    print(show(1.cmp(&2)));          // expect: -1
    print(show(2.cmp(&2)));          // expect: 0
    print(show(true.cmp(&false)));   // expect: 1
    print_bool(false.lt(&true));     // expect: true
    print_bool(3.le(&3));            // expect: true
    print_bool(3.gt(&4));            // expect: false
    print_bool(4.ge(&3));            // expect: true
    print(5.min(2));                 // expect: 2
    print(largest(3, 9, 4));         // expect: 9
    let v = largest(
        Version { major: 1, minor: 2 },
        Version { major: 1, minor: 10 },
        Version { major: 0, minor: 99 },
    );
    print(v.minor);                  // expect: 10
    print(show(Version { major: 2, minor: 0 }.cmp(&v))); // expect: 1
}
