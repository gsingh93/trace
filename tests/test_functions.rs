#[macro_use]
mod trace_test;

use trace::trace;

trace::init_depth_var!();

trace_test!(test_functions, foo(1, 2));

#[trace]
fn foo(a: i32, b: i32) {
    println!("I'm in foo!");
    bar((a, b));
}

#[trace(prefix_enter = "[ENTER]", prefix_exit = "[EXIT]")]
fn bar((a, b): (i32, i32)) -> i32 {
    println!("I'm in bar!");
    if a == 1 {
        2
    } else {
        b
    }
}
