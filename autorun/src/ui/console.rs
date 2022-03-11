use crate::{lua, configs::SETTINGS, logging::*};

#[cfg(feature = "runner")]
#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
use autorun_shared::Realm;

use std::collections::HashMap;

pub fn init() {
	unsafe {
		// Load this library before it starts spamming useless errors into our console.
		// https://github.com/Vurv78/Autorun-rs/issues/26
		winapi::um::libloaderapi::LoadLibraryA( rglua::cstr!("vaudio_speex.dll") );
		winapi::um::consoleapi::AllocConsole()
	};

	let arch = if cfg!(target_arch = "x86") {
		"x86"
	} else if cfg!(target_arch = "x86_64") {
		"x86_64"
	} else {
		"unknown"
	};

	let version = env!("CARGO_PKG_VERSION");
	println!("<====> Autorun v{version} on {arch} <====>\nType 'help' for a list of commands.");

	let hidden = SETTINGS.autorun.hide;

	std::thread::spawn(move || {
		if hidden {
			hide_console();
		}

		start();
	});
}

#[derive(Debug, thiserror::Error)]
enum CommandError {
	#[error("Error while running lua: {0}")]
	Lua(#[from] lua::RunError),

	#[error("Error while running command: {0}")]
	IO(#[from] std::io::Error),
}

type CommandArgs<'a> = std::str::Split<'a, char>;
type CommandList<'a> = HashMap<&'a str, Command<'a>>;

struct Command<'a> {
	desc: &'a str,
	func: fn(&CommandList, CommandArgs, &str) -> Result<(), CommandError>
}

macro_rules! command {
	($desc:literal, $cls:expr) => {
		Command {
			desc: $desc,
			func: $cls
		}
	}
}

fn start() {
	let mut commands: HashMap<&str, Command> = HashMap::new();

	commands.insert("help", command! (
		"Prints out all of the commands",
		|cmds, mut args, _| {
			match args.next() {
				Some(cmd_name) if !cmd_name.trim().is_empty() => {
					if let Some(cmd) = cmds.get(cmd_name) {
						println!("Help for {}:\n{}", cmd_name, cmd.desc);
					} else {
						println!("Command not found: {}", cmd_name);
					}
				},

				_ => {
					println!("[Commands]:");
					for (name, cmd) in cmds.iter() {
						println!("{}: {}", name, cmd.desc);
					}
				}
			}

			Ok(())
		}
	));

	#[cfg(feature = "runner")]
	#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
	commands.insert("lua_run_cl", command! (
		"Runs a lua script",
		|_, _, rest| {
			lua::run(Realm::Client, rest.to_owned())?;
			Ok(())
		}
	));

	#[cfg(feature = "runner")]
	#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
	commands.insert("lua_run_menu", command! (
		"Runs a lua script from the menu",
		|_, _, rest| {
			lua::run(Realm::Menu, rest.to_owned())?;
			//println!("Hello world! {rest}");
			Ok(())
		}
	));

	#[cfg(feature = "runner")]
	#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
	commands.insert("lua_openscript_menu", command! (
		"Opens a lua script from the menu",
		|_, mut args, _| {
			if let Some(script_name) = args.next() {
				let script_path = std::path::Path::new(script_name);

				if script_path.exists() {
					println!("File does not exist: {}", script_name);
					return Ok(());
				} else {
					let content = std::fs::read_to_string(script_path)?;
					lua::run(Realm::Menu, content)?;
				}
			} else {
				println!("Usage: lua_openscript_menu <script_path>");
			}

			Ok(())
		}
	));

	#[cfg(feature = "runner")]
	#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
	commands.insert("lua_openscript_cl", command! (
		"Opens a lua script from the menu",
		|_, mut args, _| {
			if let Some(script_name) = args.next() {
				let script_path = std::path::Path::new(script_name);

				if script_path.exists() {
					println!("File does not exist: {}", script_name);
					return Ok(());
				} else {
					let content = std::fs::read_to_string(script_path)?;
					lua::run(Realm::Client, content)?;
				}
			} else {
				println!("Usage: lua_openscript_cl <script_path>");
			}

			Ok(())
		}
	));

	commands.insert("settings", command! (
		"Prints out your current settings",
		|_, _, _| {
			println!("{:#?}", *SETTINGS);
			Ok(())
		}
	));

	commands.insert("hide", command! (
		"Hides the console",
		|_, _, _| {
			hide_console();
			Ok(())
		}
	));

	let mut buffer = String::new();
	loop {
		buffer.clear();

		// Loop forever in this thread, since it is separate from Gmod, and take in user input.
		match std::io::stdin().read_line(&mut buffer) {
			Err(why) => error!("{why}"),
			Ok(_) => {
				let (cmd, rest) = buffer.split_once(' ')
					.unwrap_or((buffer.trim_end(), ""));

				let rest_trim = rest.trim_end();
				let args = rest_trim.split(' ');

				if let Some(cmd) = commands.get(cmd) {
					if let Err(why) = (cmd.func)(&commands, args, rest_trim) {
						error!("{}", why);
					}
				};
			}
		}
	}
}

pub fn hide_console() {
	use std::sync::atomic::{AtomicPtr, Ordering};
	use winapi::um::{
		wincon::GetConsoleWindow,
		winuser::{ShowWindow, SW_HIDE, SW_SHOW},
	};

	let wind = unsafe { GetConsoleWindow() };
	unsafe { ShowWindow(wind, SW_HIDE) };

	match systrayx::Application::new() {
		Ok(mut app) => {
			let icon = include_bytes!("../../../assets/run.ico");
			match app.set_icon_from_buffer(icon, 32, 32) {
				Ok(_) => (),
				Err(why) => error!("Failed to set icon: {}", why),
			}

			let ptr = AtomicPtr::new(wind);

			let res = app.add_menu_item("Open", move |x| {
				let a = ptr.load(Ordering::Relaxed);
				unsafe { ShowWindow(a, SW_SHOW) };

				x.quit();
				Ok::<_, systrayx::Error>(())
			});

			match res {
				Ok(_) => {
					match app.wait_for_message() {
						Ok(_) => (),
						Err(why) => error!("Error waiting for message: {}", why),
					}
				},
				Err(why) => error!("Failed to add menu item: {why}"),
			}
		},
		Err(why) => {
			error!("Failed to create systray app! {why}")
		}
	}
}