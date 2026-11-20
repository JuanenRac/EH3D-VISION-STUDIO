//! The log of the capture: records with a level, a source and, when there is one, the code of the error, handed to a function of the program (to print them, to keep them or to send them to the
//! server). A record below the level that was asked for is never made, and a function that panics is not allowed to stop the camera.

use super::Code;
use std::sync::{Arc, Mutex};

/// How important a record is; a smaller number is more important.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
}

impl Level {
    /// The word that is written in the configuration.
    pub fn name(self) -> &'static str {
        match self {
            Level::Error => "error",
            Level::Warn => "warn",
            Level::Info => "info",
            Level::Debug => "debug",
        }
    }

    /// The level with that word.
    pub fn from_name(name: &str) -> Option<Level> {
        [Level::Error, Level::Warn, Level::Info, Level::Debug]
            .into_iter()
            .find(|l| l.name() == name)
    }
}

/// One line of the log.
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    pub level: Level,
    /// Who wrote it (`sim`, `v4l2`, `health`...).
    pub source: &'static str,
    pub code: Option<Code>,
    pub message: String,
}

type Callback = Arc<dyn Fn(&Record) + Send + Sync>;

/// A place to write records. Cloning it gives another handle to the same place.
#[derive(Clone)]
pub struct Log {
    level: Level,
    callback: Option<Callback>,
    kept: Arc<Mutex<Vec<Record>>>,
}

impl std::fmt::Debug for Log {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Log({})", self.level.name())
    }
}

/// How many records are kept in memory; the oldest go first.
pub const KEEP: usize = 1000;

impl Log {
    /// A log that keeps the records at `level` and above.
    pub fn new(level: Level) -> Log {
        Log {
            level,
            callback: None,
            kept: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// The same log with a function that gets every record that is made.
    pub fn with_callback(mut self, f: impl Fn(&Record) + Send + Sync + 'static) -> Log {
        self.callback = Some(Arc::new(f));
        self
    }

    /// The level that is kept.
    pub fn level(&self) -> Level {
        self.level
    }

    /// Writes a record when its level is wanted.
    pub fn write(&self, level: Level, source: &'static str, code: Option<Code>, message: String) {
        if level > self.level {
            return;
        }
        let record = Record {
            level,
            source,
            code,
            message,
        };
        if let Some(cb) = &self.callback {
            let cb = Arc::clone(cb);
            let r = record.clone();
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || cb(&r)));
        }
        let mut kept = self.kept.lock().unwrap_or_else(|e| e.into_inner());
        if kept.len() >= KEEP {
            kept.remove(0);
        }
        kept.push(record);
    }

    /// The records kept so far, oldest first.
    pub fn records(&self) -> Vec<Record> {
        self.kept.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn a_record_below_the_level_is_never_made() {
        let log = Log::new(Level::Warn);
        log.write(Level::Debug, "t", None, "d".into());
        log.write(Level::Info, "t", None, "i".into());
        log.write(Level::Warn, "t", Some(Code::StreamBlack), "w".into());
        log.write(Level::Error, "t", None, "e".into());
        let r = log.records();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].code, Some(Code::StreamBlack));
        assert_eq!(r[1].level, Level::Error);
    }

    #[test]
    fn the_callback_gets_each_record_and_a_clone_writes_to_the_same_place() {
        let n = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&n);
        let log = Log::new(Level::Debug).with_callback(move |_| {
            seen.fetch_add(1, Ordering::SeqCst);
        });
        let other = log.clone();
        log.write(Level::Info, "a", None, "1".into());
        other.write(Level::Info, "b", None, "2".into());
        assert_eq!(n.load(Ordering::SeqCst), 2);
        assert_eq!(log.records().len(), 2);
        assert_eq!(other.records()[1].source, "b");
    }

    #[test]
    fn a_callback_that_panics_does_not_stop_the_log() {
        let log = Log::new(Level::Info).with_callback(|_| panic!("on purpose"));
        log.write(Level::Info, "t", None, "x".into());
        assert_eq!(log.records().len(), 1);
    }

    #[test]
    fn only_the_last_records_are_kept() {
        let log = Log::new(Level::Info);
        for i in 0..KEEP + 25 {
            log.write(Level::Info, "t", None, i.to_string());
        }
        let r = log.records();
        assert_eq!(r.len(), KEEP);
        assert_eq!(r[0].message, "25");
    }

    #[test]
    fn the_levels_are_found_by_their_words_and_are_ordered() {
        for l in [Level::Error, Level::Warn, Level::Info, Level::Debug] {
            assert_eq!(Level::from_name(l.name()), Some(l));
        }
        assert!(Level::Error < Level::Debug);
        assert_eq!(Level::from_name("loud"), None);
    }
}
