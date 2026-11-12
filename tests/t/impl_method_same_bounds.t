// An impl's method may name its parameters, lifetimes and effect parameters
// as it likes, and write an effect set in any order: its bounds say what
// its trait's say.
#[derive(Discard)]
struct Up { n: u8 }

impl Iterator for Up {
    type Item = u8;
    effect e = {};

    fn next(&mut self) -> Option<u8> {
        if self.n < 3 {
            self.n = self.n + 1;
            Some(self.n)
        } else {
            None
        }
    }
}

trait Sum {
    fn sum<I: Iterator<Item = u8, e = {alloc, panic}> + Discard>(i: I) -> u8 with {alloc, panic};
    fn first<'a, J: Iterator<Item = &'a u8> + Discard>(j: J) -> u8 with J::e;
    fn call<effect g, F: FnMut() -> u8 with g + Discard>(f: F) -> u8 with g;
}

#[derive(Discard)]
struct S {}

impl Sum for S {
    fn sum<K: Iterator<Item = u8, e = {panic, alloc}> + Discard>(mut k: K) -> u8
    with {panic, alloc}
    {
        let mut s: u8 = 0;
        while let Some(v) = k.next() {
            s = s + v;
        }
        s
    }

    fn first<'b, L: Iterator<Item = &'b u8> + Discard>(mut l: L) -> u8 with L::e {
        match l.next() {
            Some(v) => *v,
            None => 0,
        }
    }

    fn call<effect h, G: FnMut() -> u8 with h + Discard>(mut g: G) -> u8 with h { g() }
}

fn one() -> u8 { 1 }

fn via<X: Sum>(_x: &X) -> u8 with {alloc, panic} {
    let a = [7u8, 8];
    X::sum(Up { n: 0 }) + X::first(a.iter()) + X::call(one)
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    out.line(&via(&S {}))?; // expect: 14
    Ok(())
}
