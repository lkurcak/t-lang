// A function using `?` still proves what it gives back itself.
fn small(n: usize) -> Option<{ v: usize | v < 4 }> {
    if n < 4 { Some(n) } else { None }
}

fn next(n: usize) -> Option<{ v: usize | v < 4 }> {
    let i = small(n)?;
    Some(i + 1)
}

fn main() {}
// error: err_soundness_free_try_result.t:8:5
