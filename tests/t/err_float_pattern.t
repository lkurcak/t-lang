fn f(x: f64) -> u8 {
    match x {
        1.5 => 1,
        // error: a float can't be a pattern, as `==` of floats isn't an equivalence; bind it and compare with `if x == 1.5`
        _ => 0,
    }
}

fn main() {}
