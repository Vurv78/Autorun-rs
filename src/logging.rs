#[cfg(feature = "logging")]
pub fn init() -> anyhow::Result<()> {
	use chrono::prelude::*;
	use std::fs::File;
	use crate::sys::statics::SAUTORUN_LOG_DIR;
	use simplelog::*;

	if !SAUTORUN_LOG_DIR.exists() {
		std::fs::create_dir_all(&*SAUTORUN_LOG_DIR)?;
	}

	let log_file_handle = File::create( SAUTORUN_LOG_DIR.join( format!("{}.log", Local::now().format("%B %d, %Y %I-%M %P") ) ) )?;

	let configs = ConfigBuilder::new()
		.set_level_color( Level::Info, Some( Color::Cyan ) )
		.set_level_color( Level::Error, Some( Color::Red ) )
		.set_level_color( Level::Warn, Some( Color::Yellow ) )
		.build();

	CombinedLogger::init(
		vec![
			TermLogger::new(
				// Logs that are level 'info' or above will be sent to the console.
				LevelFilter::Info,
				configs,
				TerminalMode::Mixed,
				ColorChoice::Auto
			),
			WriteLogger::new(
				// Logs that are level 'info' or above will be written to the log.
				LevelFilter::Info,
				Config::default(),
				log_file_handle
			)
		]
	)?;

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
	($($arg:tt)*) => { () };
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
	($($arg:tt)*) => { () };
}

// Print to stderr
#[cfg(not(feature = "logging"))]
macro_rules! error {
	($($arg:tt)*) => {
		eprintln!( $($arg)* )
	};
}

pub fn init() -> anyhow::Result<()> {
	Ok(())
}