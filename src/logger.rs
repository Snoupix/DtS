pub enum LogCategory {
    Info,
    Success,
}

macro_rules! log {
    ($title:tt, $cat:expr, $($args:tt)*) => ({
        use std::io::Write;
        use colored::Colorize;

        let title = match $cat {
            LogCategory::Info => format!("[{}]", $title).purple(),
            LogCategory::Success => format!("[{}]", $title).green(),
        };

        // Little hack to clear the stdout (current line) before writing to it.
        // Had issues with the Loading crate holding the stdout.
        print!("\r                                                                 ");
        writeln!(&mut std::io::stdout(), "\r{title} {}", format!($($args)*)).unwrap();
    })
}

pub(crate) use log;
