use crate::{configs::SETTINGS, logging::*};

mod commands;
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
					error!("{}", why);
				}
			};
		}
	}
}

pub fn hide() {
	use std::sync::atomic::{AtomicPtr, Ordering};
	use winapi::um::{
		wincon::GetConsoleWindow,
		winuser::{ShowWindow, SW_HIDE, SW_SHOW},
	};

	let wind = unsafe { GetConsoleWindow() };
	unsafe { ShowWindow(wind, SW_HIDE) };

	match systrayx::Application::new() {
		Ok(mut app) => {
			let icon = include_bytes!("../../../../assets/run.ico");
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
				Ok(_) => match app.wait_for_message() {
					Ok(_) => (),
					Err(why) => error!("Error waiting for message: {}", why),
				},
				Err(why) => error!("Failed to add menu item: {why}"),
			}
		}
		Err(why) => {
			error!("Failed to create systray app! {why}");
		}
	}
}
