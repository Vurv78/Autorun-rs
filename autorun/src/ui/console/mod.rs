use crate::{configs::SETTINGS, logging::*};

mod commands;
#[cfg(windows)]
mod tray;

pub mod palette;

use palette::{formatcol, printcol};

pub fn init() {
	unsafe {
		// Load this library before it starts spamming useless errors into our console.
		// https://github.com/Vurv78/Autorun-rs/issues/26
		let _ = libloading::Library::new("vaudio_speex");
		// winapi::um::libloaderapi::LoadLibraryA(rglua::cstr!("vaudio_speex.dll"));

		#[cfg(windows)]
		winapi::um::consoleapi::AllocConsole();
	};

	#[cfg(windows)]
	if colored::control::set_virtual_terminal(true).is_err() {
		eprintln!("Failed to enable colored output");
	}

	colored::control::set_override( SETTINGS.color_enabled() );

	let version = env!("CARGO_PKG_VERSION");
	printcol!(
		BRIGHT_BLACK,
		"<====> {} {} {} <====>",
		formatcol!(CYAN, "Autorun"),
		formatcol!(RED, bold, "v{}", version),
		formatcol!(CYAN, "on {}", formatcol!(RED, bold, "{}", std::env::consts::ARCH))
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
	#[cfg(unix)]
	{
		use winit::{
			event::{Event, WindowEvent},
			event_loop::{ControlFlow, EventLoop},
			window::WindowBuilder,
		};

		std::thread::spawn(|| {
			let event_loop = EventLoop::new();
			let window = {
				WindowBuilder::new()
				.with_title("Autorun")
				.build(&event_loop)
				.unwrap()
			};

			event_loop.run(move |event, _, control_flow| {
				*control_flow = ControlFlow::Wait;

				match event {
					Event::WindowEvent {
						event: WindowEvent::CloseRequested,
						window_id,
					} if window_id == window.id() => *control_flow = ControlFlow::Exit,
					_ => (),
				}
			});
		});
	}

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
	#[cfg(windows)]
	if let Err(why) = tray::replace_window() {
		error!("Failed to hide window: {why}");
	}
}
