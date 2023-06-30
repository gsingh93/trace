use trace::trace;

trace::init_depth_var!();

fn main() {
    foo(5, 4);
}

#[trace(format_enter = "{y} and {z} {{7}}", format_exit = "{r} * {r}")]
fn foo(z: u32, y: u32) -> u32 {
    z
}

#[cfg(test)]
#[macro_use]
mod trace_test;

#[cfg(test)]
trace_test!(test_custom_format, main());
