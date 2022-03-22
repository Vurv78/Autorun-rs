use async_fs::File;
use thiserror::Error;

use crate::configs::{self, SETTINGS};

#[derive(Error, Debug)]
pub enum LogInitError {
	#[error("Failed to create log file: {0}")]
	File(#[from] std::io::Error),
}

pub fn init() -> Result<(), LogInitError> {
	if !SETTINGS.logging.enabled {
		/*let configs = ConfigBuilder::new()
			.set_level_color(Level::Info, Some(Color::Cyan))
			.set_level_color(Level::Error, Some(Color::Red))
			.set_level_color(Level::Warn, Some(Color::Yellow))
			.set_thread_mode(ThreadLogMode::Names)
			.set_time_to_local(true)
			.set_time_format_str("%I:%M:")
			.build();

		TermLogger::init(
			LevelFilter::Info,
			configs,
			TerminalMode::Mixed,
			ColorChoice::Never,
		)?;*/

		return Ok(())
	}

	let log_dir = configs::path(configs::LOG_DIR);

	if !log_dir.exists() {
		std::fs::create_dir_all(&log_dir)?;
	}

	let log_path = log_dir.join(format!(
		"{}.log",
		chrono::Local::now().format("%B %d, %Y %I-%M %P")
	));

	// Create file synchronously since we're only gonna do this once
	let log_file_handle = std::fs::File::create(&log_path)?;

	/*let configs = ConfigBuilder::new()
		.set_level_color(Level::Info, Some(Color::Cyan))
		.set_level_color(Level::Error, Some(Color::Red))
		.set_level_color(Level::Warn, Some(Color::Yellow))
		.set_thread_mode(ThreadLogMode::Names)
		.set_time_to_local(true)
		.set_time_format_str("%I:%M:")
		.build();

	CombinedLogger::init(vec![
		TermLogger::new(
			// Logs that are level 'info' or above will be sent to the console.
			LevelFilter::Info,
			configs.clone(),
			TerminalMode::Mixed,
			ColorChoice::Never,
		),
		WriteLogger::new(
			#[cfg(debug_assertions)]
			LevelFilter::Trace,
			#[cfg(not(debug_assertions))]
			LevelFilter::Debug,
			configs,
			log_file_handle,
		)
	])?;*/

	Ok(())
}

macro_rules! warning {
	($($arg:tt)+) => {
		$crate::ui::printwarning!( normal, $($arg)+ )
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
		$crate::ui::printinfo!( normal, $($arg)+ )
	};
}
pub(crate) use info;

// Print to stderr
macro_rules! error {
	( $($arg:tt)+ ) => {
		$crate::ui::printerror!( normal, $($arg)+ )
	};
}
pub(crate) use error;

// Only prints when in a debug build.
#[cfg(debug_assertions)]
macro_rules! debug {
	( $($arg:tt)+ ) => {
		$crate::ui::printdebug!( normal, $($arg)+ )
	};
}

// We are in a release build, don't print anything.
#[cfg(not(debug_assertions))]
macro_rules! debug {
	($($arg:tt)+) => { () };
}
pub(crate) use debug;