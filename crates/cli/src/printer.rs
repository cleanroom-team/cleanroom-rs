// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::RefCell;
use std::fmt::Debug;
use std::io::Write;
use std::rc::Rc;

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

pub struct Headline(Printer);

impl Drop for Headline {
    fn drop(&mut self) {
        self.0.pop_headline();
    }
}

struct PrinterImpl {
    log_level: LogLevel,
    headlines: RefCell<Vec<String>>,
    exit_on_error: bool,
    error_count: RefCell<i32>,
    lock: RefCell<std::io::StdoutLock<'static>>,
    status_stack: RefCell<Vec<String>>,
}

#[derive(Clone, Debug)]
pub struct Printer(Rc<PrinterImpl>);

impl Debug for PrinterImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Printer")
            .field("log_level", &self.log_level)
            .finish()
    }
}

impl Printer {
    pub fn new(log_level: &LogLevel, exit_on_error: bool) -> Printer {
        let result = Self(Rc::new(PrinterImpl {
            log_level: log_level.clone(),
            headlines: RefCell::new(Default::default()),
            lock: RefCell::new(std::io::stdout().lock()),
            exit_on_error,
            error_count: RefCell::new(0),
            status_stack: RefCell::new(Vec::new()),
        }));

        if result.0.log_level != LogLevel::Off {
            result.print_status();
        }

        result
    }

    pub fn error_count(&self) -> i32 {
        *self.0.error_count.borrow()
    }

    fn print_status(&self) {
        write!(
            self.0.lock.borrow_mut(),
            "{} {}",
            ansi_term::Style::new()
                .on(ansi_term::Color::Blue)
                .fg(ansi_term::Color::White)
                .paint("Processing >>> "),
            self.0.headlines.borrow().join(" / ")
        )
        .unwrap();
        std::io::stdout().flush().unwrap();
    }

    fn clear_line(&self) {
        if self.0.log_level != LogLevel::Off {
            write!(self.0.lock.borrow_mut(), "{}\r", ansi_escapes::EraseLine).unwrap();
        }
    }

    fn print_formatted(&self, prefix: &str, message: &str) {
        if self.0.log_level == LogLevel::Off {
            return;
        }

        self.clear_line();
        for l in message.split('\n') {
            writeln!(self.0.lock.borrow_mut(), "{prefix} {l}").unwrap();
        }

        self.print_status();
    }

    fn print_headline(&self, level: usize, quiet: bool, message: &str) {
        if !quiet {
            let prefix = match level {
                1 => format!("\n{}", ansi_term::Color::Cyan.paint("*******")),
                2 => format!("{}", ansi_term::Color::Cyan.paint("+++++++")),
                3 => format!("{}", ansi_term::Color::Cyan.paint("-------")),
                _ => "-------".to_string(),
            };

            self.print_formatted(&prefix, message);
        };
    }

    #[allow(unused)]
    pub fn push_headline(&self, message: &str, quiet: bool) -> Headline {
        let headline_count = {
            let mut hls = self.0.headlines.borrow_mut();
            hls.push(message.to_string());
            hls.len()
        };
        self.print_headline(
            headline_count,
            quiet,
            &format!("{}", ansi_term::Style::new().bold().paint(message)),
        );

        Headline(self.clone())
    }

    #[allow(unused)]
    fn pop_headline(&self) {
        let mut hls = self.0.headlines.borrow_mut();
        assert!(hls.len() > 0);
        hls.pop();
    }

    #[allow(unused)]
    pub fn error(&self, message: &str) {
        if self.0.log_level >= LogLevel::Error {
            self.print_formatted(
                &format!(" {}:", ansi_term::Color::Red.paint("ERROR")),
                message,
            );
        }

        let old_count = self.0.error_count.replace_with(|&mut v| v + 1);
        if self.0.exit_on_error || old_count == std::i32::MAX / 2 {
            self.clear_line();
            std::process::exit(old_count + 1);
        }
    }

    #[allow(unused)]
    pub fn warn(&self, message: &str) {
        if self.0.log_level >= LogLevel::Warn {
            self.print_formatted(
                &format!(" {} :", ansi_term::Color::Yellow.paint("WARN")),
                message,
            );
        }
    }

    #[allow(unused)]
    pub fn info(&self, message: &str) {
        if self.0.log_level >= LogLevel::Info {
            self.print_formatted(
                &format!(" {} :", ansi_term::Color::Blue.paint("INFO")),
                message,
            );
        }
    }

    #[allow(unused)]
    pub fn debug(&self, message: &str) {
        if self.0.log_level >= LogLevel::Debug {
            self.print_formatted(
                &format!("{} :", ansi_term::Color::Cyan.paint("DEBUG")),
                message,
            );
        }
    }

    #[allow(unused)]
    pub fn push_status(&self, message: &str) {
        self.0.status_stack.borrow_mut().push(message.to_string());
        self.clear_line();
        self.print_status();
    }

    #[allow(unused)]
    pub fn pop_status(&self) {
        let status = self.0.status_stack.borrow_mut().pop();
        self.clear_line();
        self.print_status();
    }

    #[allow(unused)]
    pub fn trace(&self, message: &str) {
        if self.0.log_level >= LogLevel::Trace {
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
