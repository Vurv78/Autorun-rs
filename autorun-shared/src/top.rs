use std::fmt;

// I know this is a waste to contain 'server' but I want it to be able to be used with GetLuaInterface
#[repr(u8)]
#[derive(Clone, Copy)]
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

impl Into<u8> for Realm {
	fn into(self) -> u8 {
		match self {
			Realm::Client => 0,
			Realm::Server => 1,
			Realm::Menu => 2
		}
	}
}