use std::path::Path;
use crate::sys::{runlua::runLua, statics::*};

pub(crate) fn try_process_input() -> anyhow::Result<()> {
	// Loop forever in this thread, since it is separate from Gmod, and take in user input.
	let mut buffer = String::new();

	std::io::stdin().read_line(&mut buffer)?;
	let (word, rest) = buffer.split_once(' ').unwrap_or( (&buffer.trim_end(), "") );
	let rest_trim = rest.trim_end();

	debug!("Command used: [{}], rest [{}]", word, rest);

	match word {
		"lua_run_cl" => match runLua(REALM_CLIENT, rest.to_owned()) {
			Err(why) => error!("{}", why),
			_ => () // We don't know if it was successful yet. The code will run later in painttraverse and print there.
		},
		"lua_openscript_cl" => match std::fs::read_to_string( Path::new(rest_trim) ) {
			Err(why) => error!("Errored on lua_openscript. [{}]", why),
			Ok(contents) => match runLua( REALM_CLIENT, contents ) {
				Err(why) => error!("{}", why),
				_ => ()
			}
		},

		"lua_run_menu" => match runLua(REALM_MENU, rest.to_owned()) {
			Err(why) => error!("{}", why),
			_ => ()
		},

		"lua_openscript_menu" => match std::fs::read_to_string( Path::new( rest ) ) {
			Err(why) => error!("Errored on lua_openscript. [{}]", why),
			Ok(contents) => match runLua( REALM_MENU, contents ) {
				Err(why) => error!("Errored on lua_openscript. {}", why),
				_ => ()
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