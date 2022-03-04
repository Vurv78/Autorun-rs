use crate::{ui, hooks::{self, HookingError}, logging, plugins::{self, PluginError}};
use logging::*;

#[derive(Debug, thiserror::Error)]
pub enum StartError {
	#[cfg(feature = "logging")]
	#[error("Failed to start logger `{0}`")]
	LoggingStart(#[from] logging::LogInitError),

	#[error("Failed to hook functions `{0}`")]
	HookError(#[from] HookingError),

	#[error("Failed to start plugins `{0}`")]
	PluginError(#[from] PluginError),

	#[error("Program panicked!")]
	Panic
}

pub fn startup() -> Result<(), StartError> {
	human_panic::setup_panic!(Metadata {
		name: "Autorun".into(),
		version: env!("CARGO_PKG_VERSION").into(),
		authors: "Vurv78 <vurvdevelops@gmail.com>".into(),
		homepage: "vurv78.github.io".into(),
	});

	// Catch all potential panics to avoid crashing gmod.
	// Will simply report the error and not do anything.
	let res: Result<Result<(), StartError>, _> = std::panic::catch_unwind(|| {
		debug!("Starting: UI");
		ui::init();

		#[cfg(feature = "logging")]
		logging::init()?;

		debug!("Starting: Hooks");
		hooks::init()?;

		debug!("Starting: Plugins");
		plugins::init()?;

		debug!("Finished Startup!");

		Ok(())
	});

	match res {
		Err(_why) => Err(StartError::Panic),
		Ok(res) => res
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