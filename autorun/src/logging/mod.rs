use std::{sync::Mutex, path::PathBuf};

use once_cell::sync::Lazy;
use thiserror::Error;

use crate::configs;

#[derive(Error, Debug)]
pub enum LogInitError {
	#[error("Failed to create log file: {0}")]
	File(#[from] std::io::Error),
}

pub static LOG_PATH: Lazy<PathBuf> = Lazy::new(|| {
	let log_dir = configs::path(configs::LOG_DIR);
	log_dir.join(format!(
		"{}.log",
		chrono::Local::now().format("%Y-%M-%d")
	))
});

pub static HANDLE: Lazy<Mutex<std::fs::File>> = Lazy::new(|| {
	let log_dir = configs::path(configs::LOG_DIR);

	if !log_dir.exists() {
		std::fs::create_dir_all(&log_dir).unwrap();
	}

	let log_path = log_dir.join(format!(
		"{}.log",
		chrono::Local::now().format("%Y-%M-%d")
	));

	let opts = std::fs::OpenOptions::new()
		.create(true)
		.append(true)
		.open(log_path);

	Mutex::new( opts.expect("Failed to open file") )
});

pub fn init() -> Result<(), LogInitError> {
	once_cell::sync::Lazy::force(&HANDLE);
	Ok(())
}

macro_rules! log {
	($severity:literal, $msg:expr) => {
		let handle = std::fs::OpenOptions::new()
			.create(true)
			.append(true)
			.open(&*$crate::logging::LOG_PATH);

		match handle {
			Ok(mut handle) => {
				use std::io::Write;
				match writeln!(handle, concat!("[", $severity, "]: {}"), $msg) {
					Ok(_) => (),
					Err(why) => eprintln!("Failed to write to log file: {}", why),
				}
			},
			Err(_) => ()
		}

		/*match $crate::logging::HANDLE.try_lock() {
			Ok(mut handle) => {
				use std::io::Write;
				match writeln!(handle, concat!("[", $severity, "]: {}"), $msg) {
					Ok(_) => (),
					Err(why) => eprintln!("Failed to write to log file: {}", why),
				}
			},
			Err(_) => (),
		}*/
	}
}

pub(crate) use log;

macro_rules! warning {
	($($arg:tt)+) => {
		{
			$crate::ui::printwarning!( normal, $($arg)+ );
			$crate::logging::log!( "WARN", format!( $($arg)+ ) );
		}
	};
}

pub(crate) use warning;

macro_rules! trace {
	( $($arg:tt)+ ) => {()};
}
pub(crate) use trace;

// Regular stdout
macro_rules! info {
	( $($arg:tt)+ ) => {
		{
			$crate::ui::printinfo!( normal, $($arg)+ );
			$crate::logging::log!( "INFO", format!( $($arg)+ ) );
		}
	};
}
pub(crate) use info;

// Print to stderr
macro_rules! error {
	( $($arg:tt)+ ) => {
		{
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
		{
			$crate::ui::printdebug!( normal, $($arg)+ );
			$crate::logging::log!( "DEBUG", format!( $($arg)+ ) );
		}
	};
}

// We are in a release build, don't print anything.
#[cfg(not(debug_assertions))]
macro_rules! debug {
	($($arg:tt)+) => { () };
}

pub(crate) use debug;