// `?` is postfix, binding as tightly as a method call or a field: `-x?`
// is `-(x?)`, and `a?.b()?` is `(a?.b())?`.
#[derive(Discard, Copy)]
struct P { x: i64 }

impl P {
    fn get(self) -> Option<i64> { if self.x > 0 { Option::Some(self.x) } else { Option::None } }
}

fn p(x: i64) -> Option<P> { Option::Some(P { x }) }

fn neg(x: i64) -> Option<i64> with panic { Option::Some(-p(x)?.get()? * 2) }

fn field(x: i64) -> Option<i64> { Option::Some(p(x)?.x) }

fn main() with panic {
    print(neg(3).unwrap_or(0)); // expect: -6
    print(neg(-3).unwrap_or(0)); // expect: 0
    print(field(7).unwrap_or(0)); // expect: 7
}
