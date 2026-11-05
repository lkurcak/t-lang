// `#[derive(Debug)]` writes a struct with its name and fields, a variant
// with its name and fields, and a struct without fields by its name; each
// field by its own `Debug`, derived or written. It needs no import.
use alloc::boxed::Box;

#[derive(Debug, Discard)]
struct Point { x: i32, y: i32 }

#[derive(Debug, Discard)]
enum Shape { Empty, Circle(Point, u8), Named(&'static str) }

#[derive(Debug, Discard)]
struct Unit {}

// Recursive, through a box and through a reference.
#[derive(Debug, Discard)]
enum Chain { End, Link(i64, Box<Chain>) }

#[derive(Debug, Discard)]
struct Node<'a> { v: i32, next: Option<&'a Node<'a>> }

// Written, not derived: a field's own `Debug` is used.
#[derive(Discard)]
struct Secret { code: u32 }

impl core::fmt::Debug for Secret {
    fn fmt<W: core::fmt::Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        w.write_str("<hidden>")
    }
}

#[derive(Debug, Discard)]
struct Login<'a> { user: &'a str, secret: Secret }

// Generic: needs only what its fields need, `T: Debug` here.
#[derive(Debug, Discard)]
struct Pair<T> { both: [T; 2] }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let p = Point { x: 1, y: -2 };
    out.line(&f"{p:?}")?; // expect: Point { x: 1, y: -2 }
    let shapes = [Shape::Empty, Shape::Circle(Point { x: 0, y: 0 }, 5), Shape::Named("a\tb")];
    out.line(&f"{shapes:?}")?; // expect: [Empty, Circle(Point { x: 0, y: 0 }, 5), Named("a\tb")]
    let u = Unit {};
    out.line(&f"{u:?}")?; // expect: Unit
    let c = Chain::Link(1, Box::new(Chain::Link(2, Box::new(Chain::End))));
    out.line(&f"{c:?}")?; // expect: Link(1, Link(2, End))
    let n1 = Node { v: 1, next: Option::None };
    let n2 = Node { v: 2, next: Option::Some(&n1) };
    out.line(&f"{n2:?}")?; // expect: Node { v: 2, next: Some(Node { v: 1, next: None }) }
    let login = Login { user: "ann", secret: Secret { code: 1234 } };
    out.line(&f"{login:?}")?; // expect: Login { user: "ann", secret: <hidden> }
    let pair = Pair { both: ['a', 'b'] };
    out.line(&f"{pair:?}")?; // expect: Pair { both: ['a', 'b'] }
    let _ = login.secret.code;
    Result::Ok(())
}
