use std::{sync::atomic::{AtomicPtr, Ordering}, mem::MaybeUninit};
use winapi::um::{
	wincon::GetConsoleWindow,
	winuser::{ShowWindow, SW_HIDE, SW_SHOW, TranslateMessage, GetMessageA, DispatchMessageA},
};
use trayicon::*;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Events {
	Exit
}

pub fn replace_window() -> Result<(), trayicon::Error> {
	let wind = unsafe { GetConsoleWindow() };
	unsafe { ShowWindow(wind, SW_HIDE) };

	let ptr = AtomicPtr::new(wind);

	let (send, recv) = std::sync::mpsc::channel::<Events>();
	let icon = include_bytes!("../../../../assets/run.ico");
	let _trayicon = TrayIconBuilder::new()
		.sender(send)
		.icon_from_buffer(icon)
		.tooltip("Open Autorun 🏃")
		.menu(
			MenuBuilder::new()
				.item("Open Console", Events::Exit),
		)
		.build()?;

	let (send2, recv2) = std::sync::mpsc::channel::<bool>();

	// Event loop
	let join = std::thread::spawn(move || {
		let mut i = recv.iter();

		// Use if let since there's no other tray options right now.
		if let Some(m) = i.next() {
			match m {
				Events::Exit => {
					let window = ptr.load(Ordering::Relaxed);
					unsafe { ShowWindow(window, SW_SHOW) };
					if let Err(why) = send2.send(true) {
						error!("Failed to send exit signal: {why}");
					}
				}
			}
		}
	});

	loop {
		if let Ok(true) = recv2.try_recv() {
			if join.join().is_err() {
				error!("Failed to join thread");
			}

			break;
		}

		// Don't ask me. Windows black magic to get the message loop to work with tray icons.
		// Credit to example code from trayicon-rs.
		unsafe {
			let mut msg = MaybeUninit::uninit();
			let bret = GetMessageA(msg.as_mut_ptr(), 0 as _, 0, 0);
			if bret > 0 {
				TranslateMessage(msg.as_ptr());
				DispatchMessageA(msg.as_ptr());
			} else {
				break;
			}
		}
	}

	Ok(())
}