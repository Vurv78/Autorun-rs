use crate::{configs::SETTINGS, logging::*};

mod commands;
mod tray;
pub mod palette;

use palette::{formatcol, printcol};

pub fn init() {
	unsafe {
		// Load this library before it starts spamming useless errors into our console.
		// https://github.com/Vurv78/Autorun-rs/issues/26
		winapi::um::libloaderapi::LoadLibraryA(rglua::cstr!("vaudio_speex.dll"));
		winapi::um::consoleapi::AllocConsole()
	};

	let arch = if cfg!(target_arch = "x86") {
		"x86"
	} else if cfg!(target_arch = "x86_64") {
		"x86_64"
	} else {
		"unknown"
	};

	if colored::control::set_virtual_terminal(true).is_err() {
		eprintln!("Failed to enable colored output");
	}

	colored::control::set_override(!SETTINGS.autorun.nocolor.unwrap_or(false));

	let version = env!("CARGO_PKG_VERSION");
	printcol!(
		BRIGHT_BLACK,
		"<====> {} {} {} <====>",
		formatcol!(CYAN, "Autorun"),
		formatcol!(RED, bold, "v{}", version),
		formatcol!(CYAN, "on {}", formatcol!(RED, bold, "{}", arch))
	);

	printcol!(
		BRIGHT_RED,
		bold,
		"Type {} for a list of commands",
		formatcol!(YELLOW, bold, "{}", "help")
	);

	let hidden = SETTINGS.autorun.hide;

	std::thread::spawn(move || {
		if hidden {
			hide();
		}

		start();
	});
}

fn start() {
	let commands = commands::list();

	let mut buffer = String::new();
	loop {
		buffer.clear();

		// Loop forever in this thread, since it is separate from Gmod, and take in user input.
		if let Err(why) = std::io::stdin().read_line(&mut buffer) {
			error!("{why}");
		} else {
			let (cmd, rest) = buffer.split_once(' ').unwrap_or((buffer.trim_end(), ""));

			let rest_trim = rest.trim_end();
			let args = rest_trim.split(' ');

			if let Some(cmd) = commands.get(cmd) {
				if let Err(why) = (cmd.func)(&commands, args, rest_trim) {
					crate::ui::printerror!(normal, "{why}");
				}
			};
		}
	}
}

pub fn hide() {
	if let Err(why) = tray::replace_window() {
		error!("Failed to hide window: {why}");
	}
}
