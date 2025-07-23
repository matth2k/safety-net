/*!

  Utils for Safety Net development.

*/

/// Compare Verilog as strings up to indentation.
#[macro_export]
macro_rules! assert_verilog_eq {
    ($left:expr, $right:expr $(,)?) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                for (left_line, right_line) in left_val.lines().zip(right_val.lines()) {
                    assert_eq!(
                        left_line.trim(),
                        right_line.trim()
                    );
                }
            }
        }
    };
    ($left:expr, $right:expr, $($arg:tt)+) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                let left_string : String = left_val.into();
                let right_string : String = right_val.into();
                for (left_line, right_line) in left_val.lines().zip(right_val.lines()) {
                    assert_eq!(
                        left_line.trim(),
                        right_line.trim(),
                        std::format_args!($($arg)+)
                    );
                }
            }
        }
    };
}
