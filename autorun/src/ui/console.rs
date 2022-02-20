use crate::{lua, logging::*};
use autorun_shared::Realm;

use std::path::Path;

use indoc::printdoc;

pub fn init() {
	unsafe { winapi::um::consoleapi::AllocConsole() };

	let version = env!("CARGO_PKG_VERSION");
	printdoc!("
		<====> Autorun v{version} <====>
		Type 'help' for a list of commands.
	");

	std::thread::spawn(|| loop {
		if let Err(why) = try_process_input() {
			warn!("Failed to process console input; {}", why);
		}
	});
}

pub(crate) fn try_process_input() -> std::io::Result<()> {
	// Loop forever in this thread, since it is separate from Gmod, and take in user input.
	let mut buffer = String::new();

	std::io::stdin().read_line(&mut buffer)?;
	let (word, rest) = buffer.split_once(' ').unwrap_or((buffer.trim_end(), ""));
	let rest_trim = rest.trim_end();

	match word {
		"lua_run_cl" => {
			if let Err(why) = lua::run(Realm::Client, rest.to_owned()) {
				error!("Errored on lua_run_cl {why}");
				// We don't know if it was successful yet. The code will run later in painttraverse and print there.
			}
		}
		"lua_openscript_cl" => match std::fs::read_to_string(Path::new(rest_trim)) {
			Err(why) => error!("Errored on lua_openscript. [{}]", why),
			Ok(contents) => {
				if let Err(why) = lua::run(Realm::Client, contents) {
					error!("Errored on lua_openscript_cl: {why}");
				}
			}
		},

		"lua_run_menu" => {
			if let Err(why) = lua::run(Realm::Menu, rest.to_owned()) {
				error!("Errored on lua_run_menu {why}");
			}
		}

		"lua_openscript_menu" => match std::fs::read_to_string(Path::new(rest)) {
			Err(why) => error!("Errored on lua_openscript. [{}]", why),
			Ok(contents) => {
				if let Err(why) = lua::run(Realm::Menu, contents) {
					error!("Errored on lua_openscript. {why}");
				}
			}
		},

		"hide" => unsafe {
			use std::sync::atomic::{AtomicPtr, Ordering};
			use winapi::um::{
				wincon::GetConsoleWindow,
				winuser::{ShowWindow, SW_HIDE, SW_SHOW},
			};

			let wind = GetConsoleWindow();
			ShowWindow(wind, SW_HIDE);

			let mut tray = systrayx::Application::new().expect("Failed to create new systrayx app");
			tray.set_icon_from_buffer(&include_bytes!("../../../assets/run.ico")[..], 32, 32)
				.expect("Failed to set icon");

			let ptr = AtomicPtr::new(wind);

			tray.add_menu_item("Open", move |x| {
				let a = ptr.load(Ordering::Relaxed);
				ShowWindow(a, SW_SHOW);

				x.quit();
				Ok::<_, systrayx::Error>(())
			})
			.expect("Couldn't add menu item");

			tray.wait_for_message().unwrap();
		},

		"help" => {
			printdoc!("
				[Commands]
				---
				lua_run_cl <code>              | Runs lua code on the client state. Will print if any errors occur.
				lua_openscript_cl <file_dir>   | Runs a lua script located at file_dir, this dir being an absolute directory on your pc. (Not relative)

				lua_run_menu <code>            | Runs lua code on the menu state.
				lua_openscript_menu <code>     | Runs a lua script at an absolute path on the menu state.

				help                           | Prints this out.
				hide                           | Hides the console, but remains active.
				---
			");
		}

		msg => {
			if msg.trim().is_empty() {
				return Ok(());
			} else {
				warn!("Unknown command: {}", word);
			}
		},
	}

	Ok(())
}
