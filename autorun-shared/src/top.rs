use std::fmt;

// I know this is a waste to contain 'server' but I want it to be able to be used with GetLuaInterface
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum Realm {
	Client = 0,
	Server = 1,
	Menu = 2
}

impl fmt::Display for Realm {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", match self {
			Realm::Client => "client",
			Realm::Server => "server",
			Realm::Menu => "menu"
		})
	}
}

impl From<u8> for Realm {
	fn from(realm: u8) -> Self {
		match realm {
			0 => Realm::Client,
			1 => Realm::Server,
			2 => Realm::Menu,
			_ => panic!("Invalid realm")
		}
	}
}

impl From<Realm> for u8 {
	fn from(r: Realm) -> u8 {
		match r {
			Realm::Client => 0,
			Realm::Server => 1,
			Realm::Menu => 2
		}
	}
}