use std::path::Path;
use crate::sys::{runlua::runLua, statics::*};

pub(crate) fn try_process_input() -> anyhow::Result<()> {
	// Loop forever in this thread, since it is separate from Gmod, and take in user input.
	let mut buffer = String::new();

	std::io::stdin().read_line(&mut buffer)?;
	let (word, rest) = buffer.split_once(' ').unwrap_or( (buffer.trim_end(), "") );
	let rest_trim = rest.trim_end();

	match word {
		"lua_run_cl" => if let Err(why) = runLua(REALM_CLIENT, rest.to_owned()) {
			error!("{}", why);
			// We don't know if it was successful yet. The code will run later in painttraverse and print there.
		},
		"lua_openscript_cl" => match std::fs::read_to_string( Path::new(rest_trim) ) {
			Err(why) => error!("Errored on lua_openscript. [{}]", why),
			Ok(contents) => if let Err(why) = runLua( REALM_CLIENT, contents ) {
				error!("{}", why);
			}
		},

		"lua_run_menu" => if let Err(why) = runLua(REALM_MENU, rest.to_owned()) {
			error!("{}", why);
		},

		"lua_openscript_menu" => match std::fs::read_to_string( Path::new( rest ) ) {
			Err(why) => error!("Errored on lua_openscript. [{}]", why),
			Ok(contents) => if let Err(why) = runLua( REALM_MENU, contents ) {
				error!("Errored on lua_openscript. {}", why);
			}
		},

		"help" => {
			println!("Commands list:");
			println!("lua_run_cl <code>            | Runs lua code on the currently loaded lua state. Will print if any errors occur.");
			println!("lua_openscript_cl <file_dir> | Runs a lua script located at file_dir, this dir being an absolute directory on your pc. (Not relative)");

			println!("lua_run_menu <code>          | Runs lua code in the menu state. Will print if any errors occur.");
			println!("lua_openscript_menu <code>   | Runs lua code in the menu state. Will print if any errors occur.");

			println!("help                         | Prints this out.");
		}
		_ => ()
	}

	Ok(())
}