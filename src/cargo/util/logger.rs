use std::fmt;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use crossbeam::epoch::{self, Atomic, Owned};
use log::{self, Log, Record, Metadata};
use env_logger::{Env, Logger, WriteStyle, Builder};

use core::shell::ColorChoice;

#[derive(Clone)]
pub struct SharedLogger {
    inner: Arc<Atomic<Logger>>,
}

impl Default for SharedLogger {
    fn default() -> Self {
        SharedLogger::new(ColorChoice::Never)
    }
}

impl fmt::Debug for SharedLogger {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("SharedLogger").finish()
    }
}

fn logger(color_choice: ColorChoice) -> Logger {
    let mut logger = Builder::from_env(Env::default());

    let write_style = match color_choice {
        ColorChoice::Always => WriteStyle::Always,
        ColorChoice::CargoAuto => WriteStyle::Auto,
        ColorChoice::Never => WriteStyle::Never
    };

    logger.write_style(write_style);

    logger.build()
}

impl SharedLogger {
    pub fn new(color_choice: ColorChoice) -> Self {
        SharedLogger {
            inner: Arc::new(Atomic::new(logger(color_choice)))
        }
    }

    pub fn set_color_choice(&self, color_choice: ColorChoice) {
        self.inner.store(Some(Owned::new(logger(color_choice))), Ordering::Release);
    }

    pub fn init(&self) {
        self.with(|logger| {
            log::set_max_level(logger.filter());
            log::set_boxed_logger(Box::new(self.clone()))
        }).expect("failed to set logger")
    }

    fn with<F, R>(&self, f: F) -> R where F: FnOnce(&Logger) -> R {
        let guard = epoch::pin();

        f(&self.inner.load(Ordering::Acquire, &guard).expect("invalid logger pointer"))
    }
}

impl Log for SharedLogger {
    fn log(&self, record: &Record) {
        self.with(|logger| logger.log(record))
    }

    fn enabled(&self, metadata: &Metadata) -> bool {
        self.with(|logger| logger.enabled(metadata))
    }

    fn flush(&self) {
        self.with(|logger| logger.flush())
    }
}