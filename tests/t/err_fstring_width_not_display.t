struct P { x: i64 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let p = P { x: 1 };
    out.line(&f"{p:>8}") // error: `P` doesn't implement `Display`
}
