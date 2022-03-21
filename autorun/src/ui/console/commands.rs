use std::collections::HashMap;

#[cfg(feature = "runner")]
#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
use autorun_shared::Realm;

use crate::{configs::SETTINGS, lua};
use crate::ui::console::palette::{printcol, formatcol, printerror};

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
	pub func: fn(&CommandList, CommandArgs, &str) -> Result<(), CommandError>
}

macro_rules! command {
	($desc:literal, $cls:expr) => {
		Command {
			desc: $desc,
			func: $cls
		}
	}
}

pub fn list<'a>() -> HashMap<&'a str, Command<'a>> {
	let mut commands: HashMap<&str, Command> = HashMap::new();

	commands.insert("help", command! (
		"Prints out all of the commands",
		|cmds, mut args, _| {
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
				},

				_ => {
					println!(
						"[{}]:",
						formatcol!(CYAN, "Commands")
					);

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
					printerror!(normal, "File does not exist: {script_name}");
					return Ok(());
				} else {
					let content = std::fs::read_to_string(script_path)?;
					lua::run(Realm::Menu, content)?;
				}
			} else {
				printcol!(
					CYAN, "Usage: {} {}",
					formatcol!(YELLOW, "lua_openscript_menu"),
					formatcol!(BRIGHT_GREEN, "<script_path>")
				);
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
					let foo = 55;
					printerror!(normal, "File does not exist: {foo}");
					return Ok(());
				} else {
					let content = std::fs::read_to_string(script_path)?;
					lua::run(Realm::Client, content)?;
				}
			} else {
				printcol!(
					CYAN, "Usage: {} {}",
					formatcol!(YELLOW, "lua_openscript_cl"),
					formatcol!(GREEN, "<script_path>")
				);
			}

			Ok(())
		}
	));

	commands.insert("settings", command! (
		"Prints out your current settings",
		|_, _, _| {
			printcol!(BRIGHT_BLUE, "{:#?}", *SETTINGS);
			Ok(())
		}
	));

	commands.insert("hide", command! (
		"Hides the console",
		|_, _, _| {
			super::hide();
			Ok(())
		}
	));

	commands
}