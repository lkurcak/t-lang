// Statics are in the program's data, set before anything runs (lemma F3
// of docs/39-freestanding.md): an atomic, and a struct with frozen fields,
// a string among them, and an enum. With no std, the program says how it
// went by ending: here, well.
use core::sync::Atomic;

#[derive(Discard, Sync)]
enum Mode { Off, Limit(u32) }

#[derive(Discard, Sync)]
struct Config { name: &'static str, modes: [Mode; 2], seen: Atomic<u64>, flags: [u8; 3] }

static COUNT: Atomic<u32> = Atomic::new(7);
static CONFIG: Config = Config {
    name: "cfg",
    modes: [Mode::Off, Mode::Limit(3)],
    seen: Atomic::new(0),
    flags: [1, 2, 3],
};
static EMPTY: Atomic<bool> = Atomic::new(false);

fn main() {
    assert(COUNT.fetch_add(1) == 7);
    assert(COUNT.load() == 8);
    assert(CONFIG.name.len() == 3);
    match CONFIG.modes[1] {
        Mode::Limit(n) => assert(n == 3),
        Mode::Off => panic("off"),
    }
    let Mode::Off = CONFIG.modes[0] else { panic("on") };
    assert(CONFIG.seen.swap(5) == 0);
    assert(CONFIG.flags[2] == 3);
    assert(!EMPTY.swap(true));
    assert(EMPTY.load());
}
