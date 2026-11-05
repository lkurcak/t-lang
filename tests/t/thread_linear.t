// What a thread is given, it owns: a value that must be used up can go to
// a thread, which uses it up, or gives it back through `join`.
#[derive(Send)]
struct Ticket { n: u32 }

fn redeem(t: Ticket) -> u32 {
    let Ticket { n } = t;
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let a = Ticket { n: 3 };
    let b = Ticket { n: 4 };
    let used = os.threads.spawn(move || redeem(a));
    let back = os.threads.spawn(move || {
        let Ticket { n } = b;
        Ticket { n: n + 1 }
    });
    let n = used.join() + redeem(back.join());
    let mut out = os.stdout;
    out.line(&n)?; // expect: 8
    Ok(())
}
