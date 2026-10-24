// `<`, `<=`, `>` and `>=` are `Ord`'s: they compare anything ordered, and
// look at their operands where they are, as `==` does, moving neither.
use alloc::string::String;
use core::cmp::Ordering;

#[derive(Discard, Eq)]
struct Version { major: u32, minor: u32 }

impl Ord for Version {
    fn cmp(&self, other: &Version) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => self.minor.cmp(&other.minor),
            o => o,
        }
    }
}

fn least<T: Ord + Discard>(a: T, b: T) -> T { if b < a { b } else { a } }

fn main() with alloc {
    print_bool(false < true); // expect: true
    print_bool('a' < 'b');    // expect: true
    print_bool('é' > 'z');    // expect: true
    print_bool("abc" < "abd"); // expect: true
    print_bool("ab" >= "abc"); // expect: false
    let s = String::from("pear");
    let t = String::from("apple");
    print_bool(t < s); // expect: true
    // Neither was moved.
    print_str(s.as_str()); // expect: pear
    print_str(t.as_str()); // expect: apple
    s.free();
    t.free();
    let v1 = Version { major: 1, minor: 9 };
    let v2 = Version { major: 2, minor: 0 };
    print_bool(v1 < v2);  // expect: true
    print_bool(v2 <= v1); // expect: false
    let r1 = &v1;
    let r2 = &v2;
    print_bool(r1 >= r2); // expect: false
    print(least(v1, v2).minor); // expect: 9
    print(least(5i64, 3)); // expect: 3
}
