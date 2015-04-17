#![feature(custom_attribute, plugin)]
#![plugin(trace)]

fn main() {
    foo();
}

#[trace]
fn foo() {
    println!("I'm in foo!");
    bar();
}

#[trace(prefix_enter="[ENTER]", prefix_exit="[EXIT]")]
fn bar() {
    println!("I'm in bar!");
}
