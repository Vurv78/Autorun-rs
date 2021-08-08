use simplelog::*;

use chrono::prelude::*;
use std::fs::File;
use crate::sys::statics::SAUTORUN_LOG_DIR;

pub fn init() -> anyhow::Result<()> {
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