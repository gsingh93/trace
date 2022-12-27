trace
-----
[![Unit tests](https://github.com/gsingh93/trace/actions/workflows/tests.yml/badge.svg)](https://github.com/gsingh93/trace/actions/workflows/tests.yml)
[![Latest Version](https://img.shields.io/crates/v/trace.svg)](https://crates.io/crates/trace)
[![Documentation](https://docs.rs/trace/badge.svg)](https://docs.rs/trace)
[![License](https://img.shields.io/github/license/gsingh93/trace)](/LICENSE)

A procedural macro for tracing the execution of functions.

Adding `#[trace]` to the top of functions, `mod`s, or `impl`s will insert `println!` statements at the beginning and the end of the affected functions, notifying you of when that function was entered and exited and printing the argument and return values. Useful for quickly debugging whether functions that are supposed to be called are actually called without manually inserting print statements.

See the [`examples`](examples/) directory and the [documentation](https://docs.rs/trace) for more detail on how to use and configure this library.

## Installation

Add it as a dependency in your `Cargo.toml` file:
```toml
[dependencies]
trace = "*"
```

## Example

```rust
use trace::trace;

trace::init_depth_var!();

fn main() {
    foo(1, 2);
}

#[trace]
fn foo(a: i32, b: i32) {
    println!("I'm in foo!");
    bar((a, b));
}

#[trace(prefix_enter="[ENTER]", prefix_exit="[EXIT]")]
fn bar((a, b): (i32, i32)) -> i32 {
    println!("I'm in bar!");
    if a == 1 {
        2
    } else {
        b
    }
}
```

Output:
```
[+] Entering foo(a = 1, b = 2)
I'm in foo!
 [ENTER] Entering bar(a = 1, b = 2)
I'm in bar!
 [EXIT] Exiting bar = 2
[-] Exiting foo = ()
```
