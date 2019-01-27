extern crate trace;

use trace::trace;

trace::init_depth_var!();

fn main() {
    foo(1);
}

#[trace(pause)]
fn foo(a: i32) -> i32 {
    a
}
