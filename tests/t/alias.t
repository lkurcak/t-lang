// A type alias is another name for a type: wherever it's written, it
// means what it stands for, as if that were written there instead.
type Meters = i64;
#[derive(Discard)]
struct Two<T> { a: T, b: T }
type Pair<T> = Two<T>;
type Grid = Pair<Pair<Meters>>;
type Op = fn(i64, i64) -> i64;
type Partial = fn(i64) -> i64 with panic;

#[derive(Discard)]
struct Point { x: Meters, y: Meters }
type P = Point;

impl P {
    fn sum(&self) -> Meters { self.x + self.y }
}

fn larger(a: i64, b: i64) -> i64 { if a > b { a } else { b } }

fn halve(n: i64) -> i64 with panic {
    if n % 2 != 0 { panic("odd") }
    n / 2
}

fn apply(f: Op, a: Meters, b: Meters) -> Meters { f(a, b) }

fn swap<T>(p: Pair<T>) -> Pair<T> {
    let Two { a, b } = p;
    Two { a: b, b: a }
}

// Declared after its first use, like any item.
fn first(o: Later) -> i64 {
    match o {
        Option::Some(n) => n,
        Option::None => 0,
    }
}
type Later = Option<i64>;

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let m: Meters = 3;
    let g: Grid = Two { a: Two { a: 1, b: 2 }, b: Two { a: m, b: 4 } };
    out.line(&(g.a.a + g.a.b + g.b.a + g.b.b))?; // expect: 10
    out.line(&apply(larger, 2, 5))?; // expect: 5
    let s = swap(Two { a: 1i64, b: 2i64 });
    out.line(&(s.a * 10 + s.b))?; // expect: 21
    let h: Partial = halve;
    out.line(&h(8))?; // expect: 4
    let p: P = Point { x: 1, y: 2 };
    out.line(&p.sum())?; // expect: 3
    out.line(&first(Option::Some(5)))?; // expect: 5
    // In a block, as a `struct` can be.
    type Local = Pair<bool>;
    let l: Local = Two { a: true, b: false };
    out.line(&l.a)?; // expect: true
    Result::Ok(())
}
