// A nested `fn` shadows an outer one of the same name, and different
// functions, or blocks, can each have their own.
fn helper() -> int { 1 }

fn first() -> int {
    fn helper() -> int { 10 }
    helper()
}

fn second() -> int {
    fn helper() -> int { 20 }
    helper()
}

fn main() {
    print(helper()); // expect: 1
    print(first());  // expect: 10
    print(second()); // expect: 20
    {
        // Shadows the top-level `helper` in all of this block, even before.
        print(helper()); // expect: 30
        fn helper() -> int { 30 }
        {
            fn helper() -> int { 40 }
            print(helper()); // expect: 40
        }
        print(helper()); // expect: 30
    }
    {
        fn helper() -> int { 50 }
        print(helper()); // expect: 50
    }
    print(helper()); // expect: 1

    // A local declared before the block is shadowed by the block's `fn`,
    // and one declared in the block shadows it.
    let helper = 60;
    {
        print(helper()); // expect: 30
        fn helper() -> int { 30 }
        let helper = 70;
        print(helper); // expect: 70
    }
    print(helper); // expect: 60
}
