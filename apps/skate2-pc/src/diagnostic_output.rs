//! Routine client output shares the core's quiet flag. Fatal output bypasses it.
macro_rules! eprintln {
    ($($arg:tt)*) => {{
        if skate2_core::diagnostics::test_log::enabled() || !skate2_core::diagnostics::is_quiet() {
            let line = format!($($arg)*);
            skate2_core::diagnostics::test_log::event("client_diagnostic", format_args!("{line}"));
            if !skate2_core::diagnostics::is_quiet() { std::eprintln!("{line}"); }
        }
    }};
}
macro_rules! println {
    ($($arg:tt)*) => {{
        if !skate2_core::diagnostics::is_quiet() { std::println!($($arg)*); }
    }};
}
