use std::collections::HashMap;

#[cfg(executor)]
use autorun_shared::Realm;

use crate::{ ui::console::palette::{formatcol, printcol, printerror}, configs::SETTINGS, lua, fs as afs};
use fs_err as fs;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
	#[error("Error while running lua: {0}")]
	Lua(#[from] lua::RunError),

	#[error("Error while running command: {0}")]
	IO(#[from] std::io::Error),

	#[error("Serializing error: {0}")]
	Ser(#[from] toml::ser::Error),
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

	#[cfg(executor)]
	commands.insert(
		"lua_run_cl",
		command!("Runs a lua script", |_, _, rest| {
			lua::run_async(Realm::Client, rest.to_owned())?;
			Ok(())
		}),
	);

	#[cfg(executor)]
	commands.insert(
		"lua_run_menu",
		command!("Runs a lua script from the menu", |_, _, rest| {
			lua::run_async(Realm::Menu, rest.to_owned())?;
			Ok(())
		}),
	);

	#[cfg(executor)]
	commands.insert(
		"lua_openscript_menu",
		command!("Opens a lua script from the menu", |_, mut args, _| {
			if let Some(rawpath) = args.next() {
				let mut path = std::path::PathBuf::from(rawpath);
				if path.extension().is_none() {
					path.set_extension("lua");
				}

				if !path.exists() {
					path = afs::in_autorun(afs::INCLUDE_DIR).join(path);
				}
				let content = fs::read_to_string(path)?;
				lua::run_async(Realm::Menu, content)?;
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

	#[cfg(executor)]
	commands.insert(
		"lua_openscript_cl",
		command!("Opens a lua script from the menu", |_, mut args, _| {
			if let Some(rawpath) = args.next() {
				let mut path = std::path::PathBuf::from(rawpath);
				if path.extension().is_none() {
					path.set_extension("lua");
				}

				if !path.exists() {
					path = afs::in_autorun(afs::INCLUDE_DIR).join(path);
				}
				let content = fs::read_to_string(path)?;
				lua::run_async(Realm::Client, content)?;
			} else {
				printcol!(
					CYAN,
					"Usage: {} {}",
					formatcol!(YELLOW, "lua_openscript_cl"),
					formatcol!(BRIGHT_GREEN, "<script_path>")
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
	#[cfg(windows)]
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
			let console = unsafe {
				winapi::um::processenv::GetStdHandle(winapi::um::winbase::STD_OUTPUT_HANDLE)
			};

			let mut screen = MaybeUninit::uninit();

			unsafe {
				GetConsoleScreenBufferInfo(console, screen.as_mut_ptr());
			}

			let mut written = 0u32;
			let screen = unsafe { screen.assume_init() };

			let len_u32 = (screen.dwSize.X as u32).wrapping_mul(screen.dwSize.Y as u32);

			unsafe {
				FillConsoleOutputCharacterA(console, b' ' as i8, len_u32, top_left, &mut written);

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

	// General ``plugin`` command
	#[cfg(plugins)]
	commands.insert(
		"plugin",
		command!("General plugin command", |_, mut args, _| {
			use crate::plugins;
			if let Some(subcommand) = args.next() {
				match subcommand {
					"help" => {
						printcol!(
							WHITE,
							"[{}]:\n{}\n{}",
							formatcol!(CYAN, "Plugin Help"),
							formatcol!(
								RED,
								"Use {} to get a list of (would be) active plugins",
								formatcol!(YELLOW, "list")
							),
							formatcol!(
								RED,
								"Use {} {} to create a new plugin",
								formatcol!(YELLOW, "new"),
								formatcol!(BRIGHT_GREEN, "<name>")
							)
						);
					}

					"list" => {
						if let Ok(plugins) = plugins::find() {
							printcol!(WHITE, "[{}]:", formatcol!(CYAN, "Plugin List"));

							for (dirname, plugin) in plugins {
								printcol!(
									RED,
									"plugins/{dirname}: {}",
									match plugin {
										Ok(plugin) => {
											formatcol!(
												RED,
												// Safety 0.1.0 by Vurv
												"{} {} by {}",
												formatcol!(PURPLE, bold, "{}", plugin.get_name()),
												formatcol!(YELLOW, "{}", plugin.get_version()),
												formatcol!(BLUE, "{}", plugin.get_author())
											)
										}
										Err(why) => {
											formatcol!(WHITE, on_bright_red, "Malformed {}", why)
										}
									}
								);
							}
						} else {
							printerror!(normal, "Failed to find any plugins");
						}
					}

					"new" => {
						if let Some(plugin_name) = args.next() {
							use crate::fs as afs;

							let path = afs::FSPath::from(afs::PLUGIN_DIR).join(plugin_name);

							if plugin_name.trim().is_empty() {
								printerror!(normal, "Plugin name cannot be empty");
							} else if path.extension().is_some() {
								printerror!(
									normal,
									"Malformed plugin name (did not expect file extension)"
								);
							} else if path.exists() {
								printerror!(
									normal,
									"Cannot create plugin {}, path already exists",
									formatcol!(YELLOW, "{}", plugin_name)
								);
							} else {
								use std::io::Write;
								afs::create_dir(&path)?;

								let mut plugin_toml = afs::create_file(&path.join("plugin.toml"))?;
								let plugin_struct = crate::plugins::PluginToml {
									// There's way too much to_owned here.
									// Need to refactor the structure to use borrowed slices
									plugin: crate::plugins::PluginMetadata {
										name: plugin_name.to_owned(),
										author: "You".to_owned(),
										version: "0.1.0".to_owned(),
										description: None,
										language: Some("lua".to_owned()),
										version_required: Some(
											env!("CARGO_PKG_VERSION").to_owned(),
										),
									},
									settings: toml::Value::Table(toml::map::Map::new()),
								};
								write!(plugin_toml, "{}", toml::to_string(&plugin_struct)?)?;

								// emmylua definitions
								let mut fields = afs::create_file(&path.join("fields.lua"))?;
								write!(fields, "{}", include_str!("../../../../fields.lua"))?;

								let src = path.join("src");
								afs::create_dir(&src)?;

								let mut autorun = afs::create_file(&src.join("autorun.lua"))?;
								writeln!(autorun, "-- Autorun.log(\"Hello, autorun.lua!\")")?;

								let mut hook = afs::create_file(&src.join("hook.lua"))?;
								writeln!(hook, "-- print(\"Hello, hook.lua!\")")?;
							}
						} else {
							printcol!(
								CYAN,
								"Usage: {} {}",
								formatcol!(YELLOW, "plugin new"),
								formatcol!(BRIGHT_GREEN, "<plugin_name>")
							);
						}
					}

					other => {
						if other.trim().is_empty() {
							printcol!(
								CYAN,
								"Subcommands: [{}, {}, {}]",
								formatcol!(BRIGHT_GREEN, "help"),
								formatcol!(BRIGHT_GREEN, "list"),
								formatcol!(BRIGHT_GREEN, "new")
							);
						} else {
							printcol!(
								CYAN,
								"Unknown subcommand: {} (Should be {}, {} or {})",
								formatcol!(BRIGHT_GREEN, "{}", subcommand),
								formatcol!(BRIGHT_GREEN, "help"),
								formatcol!(BRIGHT_GREEN, "list"),
								formatcol!(BRIGHT_GREEN, "new")
							);
						}
					}
				}
			} else {
				printcol!(
					CYAN,
					"Usage: {} {}",
					formatcol!(YELLOW, "plugin"),
					formatcol!(BRIGHT_GREEN, "<subcommand>")
				);
			}
			Ok(())
		}),
	);

	commands.insert("filesteal",
		command!("General filesteal commands", |_, mut args, _| {
			match args.next() {
				Some("count") => {
					if let Ok(queue) = crate::hooks::DUMP_QUEUE.try_lock() {
						println!("{}", queue.len());
					} else {
						printerror!(normal, "Failed to lock queue");
					}
				},
				Some(_) | None => {
					printcol!(
						CYAN,
						"Usage: {} {}",
						formatcol!(YELLOW, "filesteal"),
						formatcol!(BRIGHT_GREEN, "<subcommand>")
					);
				}
			};
			Ok(())
		})
	);

	commands
}
