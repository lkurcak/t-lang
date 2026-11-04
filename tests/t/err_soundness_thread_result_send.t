// What a thread gives back goes to another thread too, so it must be
// `Send`: `Local` doesn't say it is.
#[derive(Discard)]
struct Local { n: u32 }

fn main(os: std::Os) with alloc {
    let h = os.threads.spawn(|| Local { n: 1 });
    // error: `Threads::spawn` needs its `R` to implement `Send`, and `Local` doesn't
    let _ = h.join();
}
