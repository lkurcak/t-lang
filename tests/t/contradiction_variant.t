// An arm a refinement rules out never runs: a generic enum's values may be
// thrown away when they're the variant holding nothing, and the other arm
// of its `discard` is where nothing runs (docs/81).
enum Opt<T> { No, Yes(T) }

impl<T> Opt<T> {
    #[measure]
    fn is_no(&self) -> bool {
        match self {
            Opt::No => true,
            Opt::Yes(_) => false,
        }
    }
}

fn absurd(_u: { u: () | false }) -> ! {
    loop {}
}

impl<T> Discard for { o: Opt<T> | o.is_no() } {
    fn discard(self) {
        match self {
            Opt::No => {}
            Opt::Yes(_t) => absurd(()),
        }
    }
}

struct Fd { n: i64 }

#[derive(Discard)]
enum Small { Zero, One(u32) }

impl Small {
    #[measure]
    fn is_zero(&self) -> bool {
        match self {
            Small::Zero => true,
            Small::One(_) => false,
        }
    }
}

fn get(s: { s: Small | s.is_zero() }, a: &[u32]) -> u32 with {} {
    match s {
        Small::Zero => 0,
        Small::One(_) => a[10],
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let _empty: Opt<Fd> = Opt::No;
    let full: Opt<Fd> = Opt::Yes(Fd { n: 4 });
    match full {
        Opt::Yes(fd) => {
            let Fd { n } = fd;
            out.line(&n)?; // expect: 4
        }
        Opt::No => {}
    }
    let _one = Small::One(1);
    out.line(&get(Small::Zero, &[1, 2]))?; // expect: 0
    Ok(())
}
