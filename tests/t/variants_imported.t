// Variants by their bare names (step 45): the prelude's `Some`, `None`,
// `Ok` and `Err`, and a program's own, imported with `use`. In a pattern a
// name that starts with an uppercase letter is always a path, so `None`
// there can't quietly bind a variable.
use Shape::{Dot, Empty, Rect as Box2};

#[derive(Discard)]
enum Shape {
    Empty,
    Dot(i64),
    Rect(i64, i64),
}

mod lights {
    #[derive(Discard, Copy, Eq)]
    pub enum Light {
        Red,
        Amber,
        Green,
    }

    // Variants can be passed on, as far as their enum is seen.
    pub use Light::{Amber, Green, Red};

    pub fn next(l: Light) -> Light {
        match l {
            Red => Green,
            Green => Amber,
            Amber => Red,
        }
    }
}

// A module's own names come before the prelude's: this `None` is
// `Pick::None` here.
mod own {
    #[derive(Discard)]
    pub enum Pick {
        None,
        One(i64),
    }

    use Pick::{None, One};

    pub fn picked(p: Pick) -> i64 {
        match p {
            None => 0,
            One(n) => n,
        }
    }

    pub fn none() -> Pick {
        None
    }
}

fn area(s: Shape) -> i64 {
    match s {
        Empty => 0,
        Dot(_) => 1,
        Box2(w, h) => w * h,
    }
}

fn half(n: i64) -> Option<i64> {
    if n % 2 == 0 { Some(n / 2) } else { None }
}

fn parse(d: u8) -> Result<u8, bool> {
    match d {
        b'0'..=b'9' => Ok(d - b'0'),
        _ => Err(false),
    }
}

fn first_even(xs: &[i64; 4]) -> Option<i64> {
    for x in xs.iter() {
        if let Some(h) = half(*x) {
            return Some(h * 2);
        }
    }
    None
}

fn sum_digits(s: &[u8]) -> Result<u8, bool> {
    let mut total: u8 = 0;
    for d in s.iter() {
        let Ok(v) = parse(*d) else {
            return Err(true);
        };
        total = core::num::wrapping_add(total, v);
    }
    Ok(total)
}

// A nested function's name comes before what's imported, as any name in a
// block does.
fn shadowed() -> i64 {
    fn Some(n: i64) -> i64 {
// warning: function `Some` should have a snake case name
        n + 1
    }
    Some(1)
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let total = area(Empty) + area(Dot(3)) + area(Box2(2, 5)) + area(Shape::Rect(1, 1));
    assert_eq(&total, &12);
    assert(lights::next(lights::Red) == lights::Green);
    let l = lights::next(lights::Light::Green);
    let amber = match l {
        lights::Amber => true,
        lights::Light::Red | lights::Green => false,
    };
    assert(amber);
    assert_eq(&own::picked(own::none()), &0);
    assert_eq(&own::picked(own::Pick::One(4)), &4);
    assert_eq(&first_even(&[1, 3, 8, 5]), &Some(8));
    assert_eq(&first_even(&[1, 3, 5, 7]), &None);
    let mut stack = Some(3i64);
    let mut steps = 0i64;
    while let Some(n) = stack {
        stack = if n == 0 { None } else { Some(n - 1) };
        steps = steps + 1;
    }
    assert_eq(&steps, &4);
    match sum_digits(b"123") {
        Ok(6) => {}
        Ok(_) | Err(_) => panic("wrong sum"),
    }
    assert(sum_digits(b"1x") == Err(true));
    assert_eq(&shadowed(), &2);
    let x: Option<Option<i64>> = Some(None);
    let inner = match x {
        Some(Some(_)) | None => 1i64,
        Some(None) => 2,
    };
    assert_eq(&inner, &2);
    let mut out = os.stdout;
    out.line(&f"{inner} {steps} {total}")?; // expect: 2 4 12
    Ok(())
}
