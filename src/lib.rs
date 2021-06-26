#![allow(non_snake_case)]
#[macro_use] extern crate rglua;

use std::{
	path::Path,
	thread,
	sync::mpsc
};

use once_cell::sync::OnceCell;

pub mod sys; // Sort of configurable files.
pub mod hooks;

use sys::{
	statics::*,
	runlua::runLua
};

fn try_process_input() -> anyhow::Result<()> {
	// Loop forever in this thread, since it is separate from Gmod, and take in user input.
	let mut buffer = String::new();

	std::io::stdin().read_line(&mut buffer)?;
	let (word, rest) = buffer.split_once(' ').unwrap_or( (&buffer.trim_end(), "") );

	match word {
		"lua_run" => {
			match runLua(rest) {
				Ok(_) => { println!("Ran successfully!"); }
				Err(why) => { eprintln!("{}", why); }
			}
		},
		"lua_openscript" => {
			let path = rest.trim_end();
			match std::fs::read_to_string( Path::new(path) ) {
				Err(why) => { eprintln!("Errored on lua_openscript. [{}]", why); }
				Ok(contents) => {
					match runLua( &contents ) {
						Ok(_) => { println!("Ran file {} successfully!", path) },
						Err(why) => { eprintln!("Errored when running file {}, {}", path, why); }
					}
				}
			}
		},
		"help" => {
			println!("Commands list:");
			println!("lua_run <code>            | Runs lua code on the currently loaded lua state. Will print if any errors occur.");
			println!("lua_openscript <file_dir> | Runs a lua script located at file_dir, this dir being a full directory, not relative or anything.");
			println!("help                      | Prints this out.");
		},
		"kill" => {
			// More debug than anything
			if let Some(sender) = SENDER.get() {
				sender.send(()).expect("Couldn't send mpsc kill message");
			}
		}
		_ => ()
	}

	Ok(())
}

extern "system" {
	fn AllocConsole() -> i32;
}

const SENDER: OnceCell< mpsc::Sender<()> > = OnceCell::new();

// Windows Only. I'm not going to half-ass cross-operating system support.
#[no_mangle]
pub extern "stdcall" fn DllMain(_: *const u8, reason: u32, _: *const u8) -> u32 {
	match reason {
		1 => {
			// DLL_PROCESS_ATTACH
			let (sender, receiver) = mpsc::channel();
			thread::spawn(move || {
				assert_eq!( unsafe { AllocConsole() }, 1, "Hi" );
				println!("<---> Autorun-rs <--->");
				println!("Type [help] for the list of commands");

				&*LUAL_LOADBUFFERX;

				loop {
					if let Ok(_) = try_process_input() {
						continue;
					}

					match receiver.try_recv() {
						Ok(_) => {
							break;
						},
						Err( mpsc::TryRecvError::Disconnected ) => {
							println!("Disconnected! What happened?");
							// Todo I think we have to break here as well but I kept getting this falsely running.
						},
						Err( mpsc::TryRecvError::Empty ) => ()
					}
				}
			});
			SENDER.set(sender).expect("Couldn't set mpsc kill channel!");
		}
		0 => {
			// DLL_PROCESS_DETACH
			// Detour cleanups
			unsafe {
				LUAL_LOADBUFFERX.disable().unwrap();
				if let Some(hook) = JOIN_SERVER.get() {
					hook.disable().unwrap();
				}
			};
			if let Some(sender) = SENDER.get() {
				sender.send(()).expect("Couldn't send mpsc kill message");
			}
		}
		_ => ()
	}
	1
}