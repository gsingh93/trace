extern crate env_logger;
extern crate log;

use trace::trace;

trace::init_depth_var!();

fn main() {
    env_logger::init();
    foo(1, 2);
}

#[trace(logging)]
fn foo(a: i32, b: i32) {
    println!("I'm in foo!");
}
