extern crate trace;

use trace::trace;

#[allow(non_upper_case_globals)]
static mut depth: usize = 0;

fn main() {
    foo(1);
}

#[trace(pause)]
fn foo(a: i32) -> i32 {
    a
}
