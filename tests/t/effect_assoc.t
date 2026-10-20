// Iterators set their effects; generic code passes them on.
#[derive(Discard)]
struct Ones { left: int }

impl Iterator for Ones {
    type Item = int;
    effect e = panic;

    fn next(&mut self) -> Option<int> with panic {
        if self.left == 0 {
            Option::None
        } else {
            self.left = self.left - 1;
            Option::Some(1)
        }
    }
}

fn first<I: Iterator<Item = int> + Discard>(mut it: I) -> Option<int> with I::e { it.next() }

fn main() with panic {
    print(first(Ones { left: 1 }).unwrap()); // expect: 1
    // A range's `next` can't panic, so neither can this call.
    print_bool(first(0..3).is_some()); // expect: true
    print((0..4).map(|x| x * 2).fold(0, |a, b| a + b)); // expect: 12
}
