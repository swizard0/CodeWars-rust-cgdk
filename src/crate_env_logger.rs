use std::env;
use std::io::prelude::*;
use std::io;
use std::mem;

use super::log::{Log, LogLevel, LogLevelFilter, LogRecord, SetLoggerError, LogMetadata};

mod filter {
    use std::fmt;

    pub struct Filter {
        inner: String,
    }

    impl Filter {
        #[allow(dead_code)]
        pub fn new(spec: &str) -> Result<Filter, String> {
            Ok(Filter { inner: spec.to_string() })
        }

        pub fn is_match(&self, s: &str) -> bool {
            s.contains(&self.inner)
        }
    }

    impl fmt::Display for Filter {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.inner.fmt(f)
        }
    }
}

/// Log target, either stdout or stderr.
#[derive(Debug)]
#[allow(dead_code)]
pub enum LogTarget {
    Stdout,
    Stderr,
}

/// The logger.
pub struct Logger {
    directives: Vec<LogDirective>,
    filter: Option<filter::Filter>,
    format: Box<Fn(&LogRecord) -> String + Sync + Send>,
    target: LogTarget,
}

pub struct LogBuilder {
    directives: Vec<LogDirective>,
    filter: Option<filter::Filter>,
    format: Box<Fn(&LogRecord) -> String + Sync + Send>,
    target: LogTarget,
}

impl LogBuilder {
    /// Initializes the log builder with defaults
    pub fn new() -> LogBuilder {
        LogBuilder {
            directives: Vec::new(),
            filter: None,
            format: Box::new(|record: &LogRecord| {
                format!("{}:{}: {}", record.level(),
                        record.location().module_path(), record.args())
            }),
            target: LogTarget::Stderr,
        }
    }

    /// Adds filters to the logger
    ///
    /// The given module (if any) will log at most the specified level provided.
    /// If no module is provided then the filter will apply to all log messages.
    pub fn filter(&mut self,
                  module: Option<&str>,
                  level: LogLevelFilter) -> &mut Self {
        self.directives.push(LogDirective {
            name: module.map(|s| s.to_string()),
            level: level,
        });
        self
    }

    /// Sets the format function for formatting the log output.
    ///
    /// This function is called on each record logged to produce a string which
    /// is actually printed out.
    #[allow(dead_code)]
    pub fn format<F: 'static>(&mut self, format: F) -> &mut Self
        where F: Fn(&LogRecord) -> String + Sync + Send
    {
        self.format = Box::new(format);
        self
    }

    /// Sets the target for the log output.
    ///
    /// Env logger can log to either stdout or stderr. The default is stderr.
    #[allow(dead_code)]
    pub fn target(&mut self, target: LogTarget) -> &mut Self {
        self.target = target;
        self
    }

    /// Parses the directives string in the same form as the RUST_LOG
    /// environment variable.
    ///
    /// See the module documentation for more details.
    #[allow(dead_code)]
    pub fn parse(&mut self, filters: &str) -> &mut Self {
        let (directives, filter) = parse_logging_spec(filters);

        self.filter = filter;

        for directive in directives {
            self.directives.push(directive);
        }
        self
    }

    /// Initializes the global logger with an env logger.
    ///
    /// This should be called early in the execution of a Rust program, and the
    /// global logger may only be initialized once. Future initialization
    /// attempts will return an error.
    pub fn init(&mut self) -> Result<(), SetLoggerError> {
        super::log::set_logger(|max_level| {
            let logger = self.build();
            max_level.set(logger.filter());
            Box::new(logger)
        })
    }

    /// Build an env logger.
    pub fn build(&mut self) -> Logger {
        if self.directives.is_empty() {
            // Adds the default filter if none exist
            self.directives.push(LogDirective {
                name: None,
                level: LogLevelFilter::Error,
            });
        } else {
            // Sort the directives by length of their name, this allows a
            // little more efficient lookup at runtime.
            self.directives.sort_by(|a, b| {
                let alen = a.name.as_ref().map(|a| a.len()).unwrap_or(0);
                let blen = b.name.as_ref().map(|b| b.len()).unwrap_or(0);
                alen.cmp(&blen)
            });
        }

        Logger {
            directives: mem::replace(&mut self.directives, Vec::new()),
            filter: mem::replace(&mut self.filter, None),
            format: mem::replace(&mut self.format, Box::new(|_| String::new())),
            target: mem::replace(&mut self.target, LogTarget::Stderr),
        }
    }
}

impl Logger {
    #[allow(dead_code)]
    pub fn new() -> Logger {
        let mut builder = LogBuilder::new();

        if let Ok(s) = env::var("RUST_LOG") {
            builder.parse(&s);
        }

        builder.build()
    }

    pub fn filter(&self) -> LogLevelFilter {
        self.directives.iter()
            .map(|d| d.level).max()
            .unwrap_or(LogLevelFilter::Off)
    }

    fn enabled(&self, level: LogLevel, target: &str) -> bool {
        // Search for the longest match, the vector is assumed to be pre-sorted.
        for directive in self.directives.iter().rev() {
            match directive.name {
                Some(ref name) if !target.starts_with(&**name) => {},
                Some(..) | None => {
                    return level <= directive.level
                }
            }
        }
        false
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        self.enabled(metadata.level(), metadata.target())
    }

    fn log(&self, record: &LogRecord) {
        if !Log::enabled(self, record.metadata()) {
            return;
        }

        if let Some(filter) = self.filter.as_ref() {
            if !filter.is_match(&*record.args().to_string()) {
                return;
            }
        }

        match self.target {
            LogTarget::Stdout => println!("{}", (self.format)(record)),
            LogTarget::Stderr => {
                let _ = writeln!(&mut io::stderr(), "{}", (self.format)(record));
            },
        };
    }
}

struct LogDirective {
    name: Option<String>,
    level: LogLevelFilter,
}

#[allow(dead_code)]
pub fn init() -> Result<(), SetLoggerError> {
    let mut builder = LogBuilder::new();

    if let Ok(s) = env::var("RUST_LOG") {
        builder.parse(&s);
    }

    builder.init()
}

#[allow(dead_code)]
fn parse_logging_spec(spec: &str) -> (Vec<LogDirective>, Option<filter::Filter>) {
    let mut dirs = Vec::new();

    let mut parts = spec.split('/');
    let mods = parts.next();
    let filter = parts.next();
    if parts.next().is_some() {
        println!("warning: invalid logging spec '{}', \
                 ignoring it (too many '/'s)", spec);
        return (dirs, None);
    }
    mods.map(|m| { for s in m.split(',') {
        if s.len() == 0 { continue }
        let mut parts = s.split('=');
        let (log_level, name) = match (parts.next(), parts.next().map(|s| s.trim()), parts.next()) {
            (Some(part0), None, None) => {
                // if the single argument is a log-level string or number,
                // treat that as a global fallback
                match part0.parse() {
                    Ok(num) => (num, None),
                    Err(_) => (LogLevelFilter::max(), Some(part0)),
                }
            }
            (Some(part0), Some(""), None) => (LogLevelFilter::max(), Some(part0)),
            (Some(part0), Some(part1), None) => {
                match part1.parse() {
                    Ok(num) => (num, Some(part0)),
                    _ => {
                        println!("warning: invalid logging spec '{}', \
                                 ignoring it", part1);
                        continue
                    }
                }
            },
            _ => {
                println!("warning: invalid logging spec '{}', \
                         ignoring it", s);
                continue
            }
        };
        dirs.push(LogDirective {
            name: name.map(|s| s.to_string()),
            level: log_level,
        });
    }});

    let filter = filter.map_or(None, |filter| {
        match filter::Filter::new(filter) {
            Ok(re) => Some(re),
            Err(e) => {
                println!("warning: invalid regex filter - {}", e);
                None
            }
        }
    });

    return (dirs, filter);
}
