#![feature(custom_attribute, plugin)]
#![plugin(trace)]

static mut depth: u32 = 0;

fn main() {
    foo();
}

#[trace]
fn foo() {
    println!("I'm in foo!");
    bar(0);
}

#[trace(prefix_enter="[ENTER]", prefix_exit="[EXIT]")]
fn bar(a: i32) -> i32 {
    println!("I'm in bar!");
    if a == 1 {
        0
    } else {
        1
    }
}
