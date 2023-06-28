use trace::trace;

trace::init_depth_var!();

fn main() {
    foo(4, 4)
}

#[trace(format_enter = "{y} and {x} {x}")]
fn foo(x: u32, y: u32) {

}

#[cfg(test)]
#[macro_use]
mod trace_test;

#[cfg(test)]
trace_test!(test_custom_format, main());