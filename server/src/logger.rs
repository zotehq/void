pub use log::{debug, error, info, log, trace, warn, Level};

// based on log crate error! impl
#[macro_export]
macro_rules! fatal {
    // fatal!(target: "my_target", key1 = 42, key2 = true; "a {} event", "log")
    // fatal!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => ({
        log::log!(target: $target, log::Level::Error, $($arg)+);
        $crate::save();
        std::process::exit(1);
    });

    // fatal!("a {} event", "log")
    ($($arg:tt)+) => ({
        log::log!(log::Level::Error, $($arg)+);
        $crate::save();
        std::process::exit(1);
    })
}

pub use fatal;
