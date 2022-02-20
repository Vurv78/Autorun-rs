use crate::{lua, global, logging::*};

use std::{net::{TcpListener, TcpStream}, io::{self, BufReader, Write, BufWriter}, sync::mpsc::TryRecvError};
use bincode::{config::Configuration};
use autorun_shared::{serde::{ToAutorun, ToGUI, Setting}, Realm};

pub fn init(receiver: std::sync::mpsc::Receiver<String>) {
	std::thread::spawn(|| {
		listen(receiver);
	});
}

#[derive(Debug, thiserror::Error)]
pub enum ListenError {
	#[error("IO Error when listening `{0}`")]
	IO(#[from] io::Error),

	#[error("Couldn't handle stream: `{0}`")]
	ActError(#[from] EventError),

	#[error("Failed to encode message `{0}`")]
	Serialize(#[from] bincode::error::EncodeError),
}

pub fn handle_event(evt: ToAutorun) {
	match evt {
		ToAutorun::RunLua(realm, code) => {
			if let Err(why) = lua::run(realm, code) {
				error!("Failed to run lua code: {}", why);
			}
		},
		ToAutorun::SettingChanged(setting, status) => {
			use std::sync::atomic::Ordering;

			match setting {
				Setting::Logging => {
					global::LOGGING_ENABLED.store(status, Ordering::SeqCst);
				},
				Setting::Files => {
					global::FILESTEAL_ENABLED.store(status, Ordering::SeqCst);
				}
			}
		}
	}
}

pub fn listen(receiver: std::sync::mpsc::Receiver<String>) -> Result<(), ListenError> {
	let listener = TcpListener::bind(autorun_shared::IP)?;

	// Maybe one day but this will severely complicate things
	listener.set_nonblocking(true).expect("Cannot set listener to non-blocking");

	println!("Listening....");

	/*let config = Configuration::standard();
	let mut search = true;
	loop {
		let (stream, addr) = listener.accept();

		for stream in listener.incoming() {
			match stream {
				Ok(stream) => {
					std::thread::spawn(move || {
						instance(stream, config);
					});
				},
				Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
					// wait until network socket is ready, typically implemented
					// via platform-specific APIs such as epoll or IOCP
					continue;
				}
				Err(why) => {
					error!("Failed to accept stream: {}", why);
					continue;
				}
			}
		}
	}*/

	Ok(())
}

pub fn instance(mut stream: TcpStream, config: Configuration) -> Result<(), ListenError> {
	info!("Connection established {:?}", stream);

	loop {
		match get_event(&mut stream, config) {
			Ok(x) => { handle_event(x) },
			Err(why) => {
				error!("Error in handling: {}", why);
			}
		}

		let v = bincode::encode_to_vec(ToGUI::Console("Hello from Autorun!".to_string()), config)?;
		stream.write(v.as_slice())?;
		stream.flush()?;
	}

	Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum EventError {
	#[error("IO Error when handling `{0}`")]
	IO(#[from] io::Error),

	#[error("Failed to encode message `{0}`")]
	Serialize(#[from] bincode::error::EncodeError),

	#[error("Failed to decode message `{0}`")]
	Deserialize(#[from] bincode::error::DecodeError)
}

pub fn get_event(reader: &mut TcpStream, config: Configuration) -> Result<ToAutorun, EventError> {
	let evt: ToAutorun = bincode::decode_from_std_read(reader, config)?;
	info!("Got event from remote: {:?}", evt);

	Ok(evt)
}