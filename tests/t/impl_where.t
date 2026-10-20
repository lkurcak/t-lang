// An impl's `where` clause bounds what isn't a parameter of it: here, the
// values of the iterator it holds, which it throws away.
#[derive(Discard where I: Discard)]
struct Evens<I> { iter: I }

impl<I: Iterator<Item = int>> Iterator for Evens<I> {
    type Item = int;
    effect e = {I::e, panic};

    fn next(&mut self) -> Option<int> with {I::e, panic} {
        loop {
            match self.iter.next() {
                Option::Some(x) => {
                    if x % 2 == 0 {
                        return Option::Some(x);
                    }
                },
                Option::None => return Option::None,
            }
        }
    }
}

#[derive(Discard where I: Discard)]
struct Second<I> { iter: I }

impl<I: Iterator> Iterator for Second<I>
where
    I::Item: Discard,
{
    type Item = I::Item;
    effect e = I::e;

    fn next(&mut self) -> Option<I::Item> with I::e {
        match self.iter.next() {
            Option::Some(_x) => self.iter.next(),
            Option::None => Option::None,
        }
    }
}

fn main() with panic {
    for x in (Second { iter: Evens { iter: 0..10 } }) {
        print(x);
    }
    // expect: 2
    // expect: 6
}
