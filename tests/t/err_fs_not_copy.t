// `Fs` isn't `Copy`: it's handed on, or lent, as `&Fs`.
fn main(os: std::Os) {
    let a = os.fs;
    let b = os.fs; // error: moved
    let _ = a.remove("x");
    let _ = b.remove("x");
}
