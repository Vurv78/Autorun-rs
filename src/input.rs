use std::path::Path;
use crate::sys::runLua;

pub(crate) fn try_process_input() -> anyhow::Result<()> {
	// Loop forever in this thread, since it is separate from Gmod, and take in user input.
	let mut buffer = String::new();

	std::io::stdin().read_line(&mut buffer)?;
	let (word, rest) = buffer.split_once(' ').unwrap_or( (&buffer.trim_end(), "") );

	debug!("Command used: [{}], rest [{}]", word, rest);

	match word {
		"lua_run" => {
			match runLua(rest) {
				Ok(_) => println!("Ran successfully!"),
				Err(why) => error!("{}", why)
			}
		},
		"lua_openscript" => {
			let path = rest.trim_end();
			match std::fs::read_to_string( Path::new(path) ) {
				Err(why) => error!("Errored on lua_openscript. [{}]", why),
				Ok(contents) => {
					match runLua( &contents ) {
						Ok(_) => info!("Ran file {} successfully!", path),
						Err(why) => error!("Errored when running file at path '{}'. [{}]", path, why)
					}
				}
			}
		},
		"help" => {
			println!("Commands list:");
			println!("lua_run <code>            | Runs lua code on the currently loaded lua state. Will print if any errors occur.");
			println!("lua_openscript <file_dir> | Runs a lua script located at file_dir, this dir being a full directory, not relative or anything.");
			println!("help                      | Prints this out.");
		}
		_ => ()
	}

	Ok(())
}