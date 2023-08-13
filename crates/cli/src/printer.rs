// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::RefCell;
use std::fmt::Debug;
use std::io::Write;

#[derive(Clone, Debug, Default, PartialEq, PartialOrd, clap::ValueEnum)]
pub enum LogLevel {
    Off,
    Error,
    #[default]
    Warn,
    Info,
    Debug,
    Trace,
}

pub struct Printer {
    log_level: LogLevel,
    headlines: RefCell<Vec<String>>,
    exit_on_error: bool,
    error_count: RefCell<i32>,
    lock: RefCell<std::io::StdoutLock<'static>>,
}

impl Debug for Printer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Printer")
            .field("log_level", &self.log_level)
            .finish()
    }
}

impl Printer {
    pub fn new(log_level: &LogLevel, exit_on_error: bool) -> Self {
        let result = Self {
            log_level: log_level.clone(),
            headlines: RefCell::new(Default::default()),
            lock: RefCell::new(std::io::stdout().lock()),
            exit_on_error,
            error_count: RefCell::new(0),
        };

        if result.log_level != LogLevel::Off {
            result.print_status();
        }

        result
    }

    pub fn error_count(&self) -> i32 {
        *self.error_count.borrow()
    }

    fn print_status(&self) {
        write!(
            self.lock.borrow_mut(),
            "{} {}",
            ansi_term::Style::new()
                .on(ansi_term::Color::Blue)
                .fg(ansi_term::Color::White)
                .paint("Processing >>> "),
            self.headlines.borrow().join(" / ")
        )
        .unwrap();
        std::io::stdout().flush().unwrap();
    }

    fn clear_line(&self) {
        if self.log_level != LogLevel::Off {
            write!(self.lock.borrow_mut(), "{}\r", ansi_escapes::EraseLine).unwrap();
        }
    }

    fn print_formatted(&self, prefix: &str, message: &str) {
        if self.log_level == LogLevel::Off {
            return;
        }

        self.clear_line();
        for l in message.split('\n') {
            writeln!(self.lock.borrow_mut(), "{prefix} {l}").unwrap();
        }

        self.print_status();
    }

    fn print_headline(&self, level: usize, quiet: bool, prefix: &str, message: &str) {
        let mut headlines = self.headlines.take();
        let cut_off = level.saturating_sub(1);
        assert!(headlines.len() >= cut_off);
        headlines.resize(cut_off, Default::default());
        headlines.push(message.to_string());
        self.headlines.replace(headlines);

        if !quiet {
            self.print_formatted(prefix, message);
        }
    }

    #[allow(unused)]
    pub fn h1(&self, message: &str, quiet: bool) {
        self.print_headline(
            1,
            quiet,
            &format!("\n\n{}", ansi_term::Color::Red.paint("*******")),
            &format!("{}", ansi_term::Style::new().bold().paint(message)),
        );
    }

    #[allow(unused)]
    pub fn h2(&self, message: &str, quiet: bool) {
        self.print_headline(
            2,
            quiet,
            &format!("\n{}", ansi_term::Color::Red.paint("+++++++")),
            message,
        );
    }

    #[allow(unused)]
    pub fn h3(&self, message: &str, quiet: bool) {
        self.print_headline(
            3,
            quiet,
            &format!("{}", ansi_term::Color::Green.paint("-------")),
            message,
        );
    }

    #[allow(unused)]
    pub fn error(&self, message: &str) {
        if self.log_level >= LogLevel::Error {
            self.print_formatted(
                &format!(" {}:", ansi_term::Color::Red.paint("ERROR")),
                message,
            );
        }

        let old_count = self.error_count.replace_with(|&mut v| v + 1);
        if self.exit_on_error || old_count == std::i32::MAX / 2 {
            std::process::exit(old_count + 1);
        }
    }

    #[allow(unused)]
    pub fn warn(&self, message: &str) {
        if self.log_level >= LogLevel::Warn {
            self.print_formatted(
                &format!(" {} :", ansi_term::Color::Yellow.paint("WARN")),
                message,
            );
        }
    }

    #[allow(unused)]
    pub fn info(&self, message: &str) {
        if self.log_level >= LogLevel::Info {
            self.print_formatted(
                &format!(" {} :", ansi_term::Color::Blue.paint("INFO")),
                message,
            );
        }
    }

    #[allow(unused)]
    pub fn debug(&self, message: &str) {
        if self.log_level >= LogLevel::Debug {
            self.print_formatted(
                &format!("{} :", ansi_term::Color::Cyan.paint("DEBUG")),
                message,
            );
        }
    }

    #[allow(unused)]
    pub fn trace(&self, message: &str) {
        if self.log_level >= LogLevel::Trace {
            self.print_formatted("TRACE :", message);
        }
    }

    #[allow(unused)]
    pub fn print(&self, message: &str) {
        self.print_formatted("      :", message);
    }

    #[allow(unused)]
    pub fn print_stdout(&self, message: &str) {
        self.print_formatted(
            "stdout:",
            &format!("{}", ansi_term::Color::Green.paint(message)),
        );
    }

    #[allow(unused)]
    pub fn print_stderr(&self, message: &str) {
        self.print_formatted(
            "stderr:",
            &format!("{}", ansi_term::Color::Red.paint(message)),
        );
    }
}

impl Drop for Printer {
    fn drop(&mut self) {
        self.clear_line();
    }
}
