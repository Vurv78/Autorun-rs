use std::path::PathBuf;

use fs_err as fs;
use once_cell::sync::Lazy;
use thiserror::Error;

use crate::fs::{self as afs, LOG_DIR};

#[derive(Error, Debug)]
pub enum LogInitError {
	#[error("Failed to create log file: {0}")]
	File(#[from] std::io::Error),
}

pub static LOG_PATH: Lazy<PathBuf> = Lazy::new(|| {
	afs::in_autorun(LOG_DIR).join(format!("{}.log", chrono::Local::now().format("%Y-%m-%d")))
});

pub fn init() -> Result<(), LogInitError> {
	let handle = fs::OpenOptions::new()
		.create(true)
		.append(true)
		.open(&*LOG_PATH);

	if let Ok(mut handle) = handle {
		use std::io::Write;
		if let Err(why) = writeln!(
			handle,
			"[INFO]: Logging started at {}\n",
			chrono::Local::now()
		) {
			eprintln!("Failed to write initial log message {why}");
		}
	}

	Ok(())
}

macro_rules! log {
	($severity:literal, $msg:expr) => {
		let handle = fs_err::OpenOptions::new()
			.create(true)
			.append(true)
			.open(&*$crate::logging::LOG_PATH);

		match handle {
			Ok(mut handle) => {
				use std::io::Write;
				let _ = writeln!(handle, concat!("[", $severity, "]: {}"), $msg);
			}
			Err(why) => {
				eprintln!("Failed to open log file: {why}");
			}
		}
	};
}

pub(crate) use log;

macro_rules! warning {
	($($arg:tt)+) => {
		if $crate::configs::SETTINGS.logging.enabled {
			$crate::ui::printwarning!( normal, $($arg)+ );
			$crate::logging::log!( "WARN", format!( $($arg)+ ) );
		}
	};
}

pub(crate) use warning;

macro_rules! trace {
	( $($arg:tt)+ ) => {
		()
	};
}
pub(crate) use trace;

macro_rules! info {
	( $($arg:tt)+ ) => {
		if $crate::configs::SETTINGS.logging.enabled {
			$crate::ui::printinfo!( normal, $($arg)+ );
			$crate::logging::log!( "INFO", format!( $($arg)+ ) );
		}
	};
}
pub(crate) use info;

// Print to stderr
macro_rules! error {
	( $($arg:tt)+ ) => {
		if $crate::configs::SETTINGS.logging.enabled {
			$crate::ui::printerror!( normal, $($arg)+ );
			$crate::logging::log!( "ERROR", format!( $($arg)+ ) );
		}
	};
}
pub(crate) use error;

// Only prints when in a debug build.
#[cfg(debug_assertions)]
macro_rules! debug {
	( $($arg:tt)+ ) => {
		if $crate::configs::SETTINGS.logging.enabled {
			$crate::ui::printdebug!( normal, $($arg)+ );
			$crate::logging::log!( "DEBUG", format!( $($arg)+ ) );
		}
	};
}

// We are in a release build, don't print anything.
#[cfg(not(debug_assertions))]
macro_rules! debug {
	($($arg:tt)+) => {
		()
	};
}

pub(crate) use debug;
