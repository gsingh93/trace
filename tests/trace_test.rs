extern crate gag;

#[macro_export]
macro_rules! trace_test {
    ($test_name:ident, $expression:expr) => {
        #[test]
        fn $test_name() {
            use std::io::Read;

            let mut actual_output = String::new();
            {
                let mut buf = gag::BufferRedirect::stdout().unwrap();
                $expression;
                buf.read_to_string(&mut actual_output).unwrap();
            }

            let test_filename = concat!("tests/", stringify!($test_name), ".expected");
            let expected_output = std::fs::read_to_string(test_filename).unwrap();
            assert_eq!(actual_output, expected_output);
        }
    };
}
