extern crate env_logger;
extern crate log;
extern crate trace;

#[macro_use]
mod trace_test;

use trace::trace;

trace::init_depth_var!();

trace_test!(test_logging, {
    env_logger::init();
    foo(1, 2);
});

#[trace(logging)]
fn foo(a: i32, b: i32) {
    println!("I'm in foo!");
}
