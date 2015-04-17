trace [![](https://meritbadge.herokuapp.com/trace)](https://crates.io/crates/trace)
-----

A syntax extension for tracing the execution of functions. Adding `#[trace]` to the top of any function will insert `println!` statements at the beginning and end of that function, notifying you of when that function was entered and exited. This is useful for quickly debugging whether functions that are supposed to be called are actually called without manually inserting print statements.

## Installation

Add `trace = "*"` to your `Cargo.toml`.

## Example

Here is an example you can find in the examples folder. If you've cloned the project, you can run this with `cargo run --example example`.

```
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
```

Output:
```
[+] Entering foo
I'm in foo!
[ENTER] Entering bar
I'm in bar!
[EXIT] Exiting bar
[-] Exiting foo
```

Note that you can customize the prefix of the `println!` statement with `prefix_enter` and `prefix_exit`.
