use crate::{
	hooks::{self, HookingError},
	logging,
	ui
};

#[cfg(plugins)]
use crate::plugins::{self, PluginError};

#[cfg(http)]
use crate::version;

use fs_err as fs;
use logging::*;

#[derive(Debug, thiserror::Error)]
pub enum StartError {
	#[error("Failed to start logger `{0}`")]
	LoggingStart(#[from] logging::LogInitError),

	#[error("Failed to hook functions `{0}`")]
	HookError(#[from] HookingError),

	#[cfg(plugins)]
	#[error("Failed to start plugins `{0}`")]
	PluginError(#[from] PluginError),

	#[error("Program panicked!")]
	Panic,

	#[error("Failed to create essential directory `{0}`")]
	IO(#[from] std::io::Error),
}

pub fn startup() -> Result<(), StartError> {
	// Catch all potential panics to avoid crashing gmod.
	// Will simply report the error and not do anything.
	let res: Result<Result<(), StartError>, _> = std::panic::catch_unwind(|| {
		use crate::fs as afs;

		// <USER>/autorun/
		let base = afs::base();
		if !base.exists() {
			fs::create_dir(&base)?;
		}

		// Make sure all essential directories exist
		for p in [
			afs::INCLUDE_DIR,
			afs::LOG_DIR,
			afs::BIN_DIR,
			afs::DUMP_DIR,
			afs::PLUGIN_DIR,
		] {
			let path = base.join(p);
			if !path.exists() {
				fs::create_dir(&path)?;
			}
		}

		// Make sure settings exist or create them
		// If invalid, will panic inside of here to pass the error to the user anyway.
		once_cell::sync::Lazy::force(&crate::configs::SETTINGS);

		logging::init()?;

		debug!("Starting: UI");
		ui::init();

		debug!("Starting: Hooks");
		hooks::init()?;

		#[cfg(plugins)]
		{
			debug!("Starting: Plugins");
			plugins::init()?;
		}

		debug!("Finished Startup!");

		#[cfg(http)]
		version::check();

		Ok(())
	});

	match res {
		Err(_why) => Err(StartError::Panic),
		Ok(res) => res,
	}
}

#[derive(Debug, thiserror::Error)]
pub enum CleanupError {
	#[error("Failed to unhook functions '{0}'")]
	HookError(#[from] detour::Error),
}

pub fn cleanup() -> Result<(), CleanupError> {
	hooks::cleanup()?;

	Ok(())
}
