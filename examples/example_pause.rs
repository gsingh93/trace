extern crate trace;

use trace::trace;

static mut DEPTH: usize = 0;

fn main() {
    foo(1);
}

#[trace(pause)]
fn foo(a: i32) -> i32 {
    a
}
