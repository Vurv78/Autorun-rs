use simplelog::*;
use std::fs::File;
use thiserror::Error;

use crate::configs::{self, SETTINGS};

#[cfg(not(feature = "logging"))]
mod fallback;

#[cfg(not(feature = "logging"))]
pub use fallback::*;

#[cfg(feature = "logging")]
pub use log::{info, error, warn, debug, trace};

#[derive(Error, Debug)]
pub enum LogInitError {
	#[error("{0}")]
	#[cfg(feature = "logging")]
	Logger(#[from] log::SetLoggerError),

	#[error("Failed to create log file: {0}")]
	File(#[from] std::io::Error),
}

#[cfg(feature = "logging")]
pub fn init() -> Result<(), LogInitError> {
	if !SETTINGS.logging.enabled {
		let configs = ConfigBuilder::new()
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
		)?;

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

	let log_file_handle = File::create(&log_path)?;

	let configs = ConfigBuilder::new()
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
	])?;

	Ok(())
}