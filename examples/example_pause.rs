#![feature(custom_attribute, plugin)]
#![plugin(trace)]

static mut DEPTH: u32 = 0;

fn main() {
    foo(1);
}

#[trace(pause)]
fn foo(a: i32) -> i32 {
    a
}
