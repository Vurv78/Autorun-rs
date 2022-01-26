// Shared structs for communication between the UI and the DLL.
use bincode::{Decode, Encode};

#[derive(Debug, Encode, Decode)]
pub enum Setting {
	Logging,
	Files
}

/// UI -> Autorun
#[derive(Debug, Encode, Decode)]
pub enum ToAutorun {
	// User wants to run lua.
	RunLua(crate::Realm, String),

	// For now settings are just toggles.
	// In the future there may be sliders and whatnot
	SettingChanged(Setting, bool),

	// Update the GUI Console with data from Autorun stdout.
	SyncConsole
}

/// Autorun -> UI
#[derive(Decode, Encode)]
pub enum ToGUI {
	// Need to log something to the user's console
	Console(String),
}