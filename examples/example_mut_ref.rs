extern crate trace;

use trace::trace;

trace::init_depth_var!();

fn main() {
    let mut a = 10;
    let mut b = 20;
    foo(&mut a, &mut b);
}

#[trace]
fn foo(a: &mut u32, b: &mut u32) {
    *a += 20;
    *b += 40;
    bar(a);
    bar(b);
}

#[trace]
fn bar(x: &mut u32) {
    *x -= 5;
}
