use std::collections::HashMap;

#[cfg(feature = "runner")]
#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
use autorun_shared::Realm;

use crate::ui::console::palette::{formatcol, printcol, printerror};
use crate::{configs::SETTINGS, lua};
use fs_err as fs;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
	#[error("Error while running lua: {0}")]
	Lua(#[from] lua::RunError),

	#[error("Error while running command: {0}")]
	IO(#[from] std::io::Error),
}

type CommandArgs<'a> = std::str::Split<'a, char>;
type CommandList<'a> = HashMap<&'a str, Command<'a>>;

pub struct Command<'a> {
	pub desc: &'a str,
	pub func: fn(&CommandList, CommandArgs, &str) -> Result<(), CommandError>,
}

macro_rules! command {
	($desc:literal, $cls:expr) => {
		Command {
			desc: $desc,
			func: $cls,
		}
	};
}

pub fn list<'a>() -> HashMap<&'a str, Command<'a>> {
	let mut commands: HashMap<&str, Command> = HashMap::new();

	commands.insert(
		"help",
		command!("Prints out all of the commands", |cmds, mut args, _| {
			match args.next() {
				Some(cmd_name) if !cmd_name.trim().is_empty() => {
					if let Some(cmd) = cmds.get(cmd_name) {
						printcol!(
							CYAN,
							italic,
							"Help for {}:\n{}",
							formatcol!(YELLOW, bold, "{cmd_name}"),
							formatcol!(BRIGHT_GREEN, "{}", cmd.desc)
						);
					} else {
						printerror!(
							normal,
							"Command not found: {}",
							formatcol!(YELLOW, bold, "{cmd_name}")
						);
					}
				}

				_ => {
					println!("[{}]:", formatcol!(CYAN, "Commands"));

					for (name, cmd) in cmds.iter() {
						println!(
							"{}: {}",
							formatcol!(YELLOW, bold, "{}", name),
							formatcol!(BRIGHT_GREEN, "{}", cmd.desc)
						);
					}
				}
			}

			Ok(())
		}),
	);

	#[cfg(feature = "runner")]
	#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
	commands.insert(
		"lua_run_cl",
		command!("Runs a lua script", |_, _, rest| {
			lua::run(Realm::Client, rest.to_owned())?;
			Ok(())
		}),
	);

	#[cfg(feature = "runner")]
	#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
	commands.insert(
		"lua_run_menu",
		command!("Runs a lua script from the menu", |_, _, rest| {
			lua::run(Realm::Menu, rest.to_owned())?;
			Ok(())
		}),
	);

	#[cfg(feature = "runner")]
	#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
	commands.insert(
		"lua_openscript_menu",
		command!("Opens a lua script from the menu", |_, mut args, _| {
			if let Some(script_name) = args.next() {
				let script_path = std::path::Path::new(script_name);

				if script_path.exists() {
					printerror!(normal, "File does not exist: {script_name}");
					return Ok(());
				} else {
					let content = fs::read_to_string(script_path)?;
					lua::run(Realm::Menu, content)?;
				}
			} else {
				printcol!(
					CYAN,
					"Usage: {} {}",
					formatcol!(YELLOW, "lua_openscript_menu"),
					formatcol!(BRIGHT_GREEN, "<script_path>")
				);
			}

			Ok(())
		}),
	);

	#[cfg(feature = "runner")]
	#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
	commands.insert(
		"lua_openscript_cl",
		command!("Opens a lua script from the menu", |_, mut args, _| {
			if let Some(script_name) = args.next() {
				let script_path = std::path::Path::new(script_name);

				if script_path.exists() {
					let content = fs::read_to_string(script_path)?;
					lua::run(Realm::Client, content)?;
				} else {
					printerror!(normal, "File does not exist: {script_name}");
					return Ok(());
				}
			} else {
				printcol!(
					CYAN,
					"Usage: {} {}",
					formatcol!(YELLOW, "lua_openscript_cl"),
					formatcol!(GREEN, "<script_path>")
				);
			}

			Ok(())
		}),
	);

	commands.insert(
		"settings",
		command!("Prints out your current settings", |_, _, _| {
			printcol!(BRIGHT_BLUE, "{:#?}", *SETTINGS);
			Ok(())
		}),
	);

	commands.insert(
		"hide",
		command!("Hides the console", |_, _, _| {
			super::hide();
			Ok(())
		}),
	);

	// Credit: https://stackoverflow.com/a/6487534/14076600
	// I had no idea clearing console was this bad on windows..
	commands.insert(
		"clear",
		command!("Clears the console", |_, _, _| {
			use std::mem::MaybeUninit;
			use winapi::um::{
				wincon::{
					FillConsoleOutputAttribute, FillConsoleOutputCharacterA,
					GetConsoleScreenBufferInfo, SetConsoleCursorPosition, FOREGROUND_BLUE,
					FOREGROUND_GREEN, FOREGROUND_RED,
				},
				wincontypes::COORD,
			};

			let top_left = COORD { X: 0, Y: 0 };
			let console = unsafe { winapi::um::processenv::GetStdHandle(winapi::um::winbase::STD_OUTPUT_HANDLE) };

			let mut screen = MaybeUninit::uninit();

			unsafe {
				GetConsoleScreenBufferInfo(console, screen.as_mut_ptr());
			}

			let mut written = 0u32;
			let screen = unsafe { screen.assume_init() };

			let len_u32 = (screen.dwSize.X as u32).wrapping_mul(screen.dwSize.Y as u32);

			unsafe {
				FillConsoleOutputCharacterA(
					console,
					b' ' as i8,
					len_u32,
					top_left,
					&mut written,
				);

				FillConsoleOutputAttribute(
					console,
					FOREGROUND_GREEN | FOREGROUND_RED | FOREGROUND_BLUE,
					len_u32,
					top_left,
					&mut written,
				);

				SetConsoleCursorPosition(console, top_left);
			}

			Ok(())
		}),
	);

	commands
}
