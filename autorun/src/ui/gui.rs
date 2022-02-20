use std::{io::Read, time::Duration};
use autorun_shared::serde::{ToGUI, ToAutorun, Setting};
use message_io::{node::{self, NodeEvent}, network::{Transport, NetEvent}};
use crate::{lua, logging::error, global::{FILESTEAL_ENABLED, LOGGING_ENABLED}};

pub fn init() {
	unsafe { winapi::um::consoleapi::AllocConsole() };

	let stdout = shh::stdout().expect("Failed to override stdout");
	std::thread::spawn(move || {
		instance(stdout);
	});
}

enum Signal {
	Greet,
	Msg(String)
}

pub fn instance(mut stdout: shh::ShhStdout) {
	//let connection = std::net::TcpListener::bind("127.0.0.1:8080").unwrap();
	// connection.set_nonblocking(true).expect("Couldn't set nonblocking");

	let (handler, listener) = node::split::<()>();
	handler.network().listen(Transport::FramedTcp, autorun_shared::IP).unwrap();

	use std::sync::{Arc, Mutex};
	// let endpoints = vec![];
	let endpoints = Arc::new(Mutex::new(vec![]));
	let for_listener = Arc::clone(&endpoints);

	let config = bincode::config::Configuration::standard();
	listener.for_each(move |event| match event.network() {
		NetEvent::Connected(_, _ok) => unreachable!(),
		NetEvent::Accepted(endpoint, _) => {
			for_listener.lock().unwrap().push(endpoint);

			let a = bincode::encode_to_vec(ToGUI::Console("Hello".to_string()), config).unwrap();
			handler.network().send(endpoint, a.as_slice());
			println!("Sent Hello to {endpoint}");
		},
		NetEvent::Message(endpoint, data) => {
			eprintln!("Received: {}", data.len());

			let message: (ToAutorun, usize) = bincode::decode_from_slice(data, config).unwrap();
			match message.0 {
				ToAutorun::RunLua(realm, lua) => {
					if let Err(why) = lua::run(realm, lua) {
						error!("Error running lua: {}", why);
					}
				},
				ToAutorun::SettingChanged(setting, value) => {
					use std::sync::atomic::Ordering;
					eprintln!("Setting changed {setting:?} {value}");
					match setting {
						Setting::Logging => {
							LOGGING_ENABLED.store(value, Ordering::SeqCst);
						},
						Setting::Files => {
							FILESTEAL_ENABLED.store(value, Ordering::SeqCst);
						}
					}
				},
				ToAutorun::SyncConsole => {
					eprintln!("Requested a console sync");
					let mut buf = String::new();
					let _read = stdout.read_to_string(&mut buf).unwrap();

					let payload = bincode::encode_to_vec( ToGUI::Console(buf), config ).unwrap();
					handler.network().send(endpoint, payload.as_slice());
				}
			}
		},
		NetEvent::Disconnected(endpoint) => {
			endpoints.lock().unwrap().retain(|e| e != &endpoint);
		},
	});

	/*loop {
		match connection.accept() {
			Ok((mut stream, _addr)) => {
				stream.set_nonblocking(true).expect("Failed to set stream nonblocking");

				// let mut stderr = std::io::stderr();
				loop {
					let mut buf = Vec::new();
					match stdout.read_to_end(&mut buf) {
						Ok(len) => {
							if len != 0 {
								eprintln!("Writing {len}");
								stream.write_all(&buf).expect("Failed to write");
							}
						},
						Err(why) => {
							eprintln!("Stdout error: {why}")
						}
					};
				}
			}
			Err(why) => {
				eprintln!("Couldn't connect to client: {}", why);
			}
		};
	}*/
}