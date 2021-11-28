use thiserror::Error;

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
	use crate::sys::statics::SAUTORUN_LOG_DIR;
	use simplelog::*;
	use std::fs::File;

	if !SAUTORUN_LOG_DIR.exists() {
		std::fs::create_dir_all(&*SAUTORUN_LOG_DIR)?;
	}

	let log_file_handle = File::create(SAUTORUN_LOG_DIR.join(format!(
		"{}.log",
		chrono::Local::now().format("%B %d, %Y %I-%M %P")
	)))?;

	let configs = ConfigBuilder::new()
		.set_level_color(Level::Info, Some(Color::Cyan))
		.set_level_color(Level::Error, Some(Color::Red))
		.set_level_color(Level::Warn, Some(Color::Yellow))
		.build();

	CombinedLogger::init(vec![
		TermLogger::new(
			// Logs that are level 'info' or above will be sent to the console.
			LevelFilter::Info,
			configs,
			TerminalMode::Mixed,
			ColorChoice::Auto,
		),
		WriteLogger::new(
			#[cfg(debug_assertions)]
			LevelFilter::Debug,
			#[cfg(not(debug_assertions))]
			LevelFilter::Info,
			Config::default(),
			log_file_handle,
		),
	])?;

	Ok(())
}

// Stderr
#[cfg(not(feature = "logging"))]
macro_rules! warn {
	($($arg:tt)*) => {
		eprintln!( $($arg)* )
	};
}

// Will never print (unless logging is enabled)
#[cfg(not(feature = "logging"))]
macro_rules! trace {
	($($arg:tt)*) => {
		()
	};
}

// Regular stdout
#[cfg(not(feature = "logging"))]
macro_rules! info {
	($($arg:tt)*) => {
		println!( $($arg)* )
	};
}

// Only prints when in a debug build.
#[cfg(all(not(feature = "logging"), debug_assertions))]
macro_rules! debug {
	($($arg:tt)*) => {
		println!( $($arg)* )
	};
}

// We are in a release build, don't print anything.
#[cfg(all(not(feature = "logging"), not(debug_assertions)))]
macro_rules! debug {
	($($arg:tt)*) => {
		()
	};
}

// Print to stderr
#[cfg(not(feature = "logging"))]
macro_rules! error {
	($($arg:tt)*) => {
		eprintln!( $($arg)* )
	};
}

#[cfg(not(feature = "logging"))]
pub fn init() -> Result<(), LogInitError> {
	Ok(())
}
