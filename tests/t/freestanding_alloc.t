// A program that needs nothing of std, so it builds for the freestanding
// target too (docs/39-freestanding.md): its heap is the platform's, and
// what it shows, it shows by how it ends. Lists, boxes, strings and arenas
// all get their memory through the platform's functions, and give it back
// with the size and alignment they got it with.
use alloc::arena::Arena;
use alloc::boxed::Box;
use alloc::list::List;
use alloc::string::String;

#[derive(Discard)]
struct Node<'a> { value: u32, next: Option<&'a Node<'a>> }

// More values than an arena's first 8 slots, so it grows its list of
// them; dropped with 20 values in 32 slots.
fn chain() -> u32 {
    let nodes = Arena::new();
    let mut last: &Node = nodes.alloc(Node { value: 0, next: Option::None });
    for i in 1..20u32 { last = nodes.alloc(Node { value: i, next: Option::Some(last) }) }
    let mut sum = 0u32;
    let mut at = Option::Some(last);
    while let Option::Some(n) = at {
        sum = sum + n.value;
        at = n.next;
    }
    sum + nodes.len() as u32
}

fn main() {
    let mut l = List::new();
    for i in 0..1000u32 { l.push(i) }
    let mut total = 0u64;
    for x in l.iter() { total = total + (*x as u64) }
    while l.len() > 10 { let _ = l.pop(); }
    let b = Box::new(total);
    let mut s = String::of(&*b);
    s.push(' ');
    let n = String::of(&chain());
    s.push_str(n.as_str());
    core::panic::panic(s.as_str())
}
// error: runtime error: panicked: 499500 210
