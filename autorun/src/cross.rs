use crate::{ui, global, hooks, logging};
use logging::*;
use rglua::prelude::*;

#[derive(Debug, thiserror::Error)]
pub enum StartError {
	#[cfg(feature = "logging")]
	#[error("Failed to start logger `{0}`")]
	LoggingStart(#[from] logging::LogInitError),

	#[error("Failed to hook functions `{0}`")]
	HookError(#[from] detour::Error),

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

#[derive(Debug, thiserror::Error)]
pub enum MenuStartError {
	#[error("Failed to find JoinServer. Corrupted gmod install?")]
	NoJoinServer,

	#[error("Failed to detour JoinServer {0}")]
	DetourError(#[from] detour::Error),
}

// When menu state is ready, run this.
pub fn startup_menu(l: LuaState) -> Result<(), MenuStartError> {
	use std::sync::atomic::AtomicPtr;

	if global::MENU_STATE.set(AtomicPtr::from(l)).is_err() {
		error!("MENU_STATE was occupied in gmod13_open. Shouldn't happen.");
	}

	lua_getglobal(l, cstr!("JoinServer"));
	let joinserver_fn = lua_tocfunction(l, -1);
	lua_pop(l, 1);

	unsafe {
		let hook = detour::GenericDetour::new(joinserver_fn, hooks::joinserver_h)?;
		hooks::JOINSERVER_H.set(hook).unwrap();
		hooks::JOINSERVER_H.get().unwrap().enable().unwrap();
	}

	printgm!(l, "Loaded Autorun!");

	Ok(())
}
