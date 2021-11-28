use atomic::Atomic;
use once_cell::sync::{Lazy, OnceCell};
use std::ffi::c_void;
use std::{
	path::PathBuf,
	sync::{
		atomic::{AtomicBool, AtomicPtr},
		Arc, Mutex,
	},
};

// ---------------- Configs ---------------- //
pub static HOME_DIR: Lazy<PathBuf> =
	Lazy::new(|| home::home_dir().expect("Couldn't get your home directory!"));
pub static SAUTORUN_DIR: Lazy<PathBuf> = Lazy::new(|| HOME_DIR.join("sautorun-rs"));
#[cfg(feature = "logging")]
pub static SAUTORUN_LOG_DIR: Lazy<PathBuf> = Lazy::new(|| SAUTORUN_DIR.join("logs"));
pub static SAUTORUN_SCRIPT_DIR: Lazy<PathBuf> = Lazy::new(|| SAUTORUN_DIR.join("scripts"));

// This location is run right before autorun.
pub static AUTORUN_SCRIPT_PATH: Lazy<PathBuf> = Lazy::new(|| (*SAUTORUN_DIR).join("autorun.lua"));

// Basically ROC, whenever a lua script is ran, run this and pass the code. If it returns true or nil, run the code, else don't
pub static HOOK_SCRIPT_PATH: Lazy<PathBuf> = Lazy::new(|| (*SAUTORUN_DIR).join("hook.lua"));

// ---------------- Configs ---------------- //

// No more static mut! 🥳

// AtomicPtr automatically attaches *mut to the type given. That's why we give CVoid instead of LuaState, because we'll end up with *mut CVoid aka LuaState
pub static CLIENT_STATE: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut()); // Not using AtomicPtr::default() because it isn't a static function
pub static CURRENT_SERVER_IP: Atomic<&'static str> = Atomic::new("unknown_ip"); // Using Atomic crate because there is no official way to get an atomic str / string.

pub static HAS_AUTORAN: AtomicBool = AtomicBool::new(false); // Whether an autorun script has been run and detected already.
pub static MENU_STATE: OnceCell<AtomicPtr<c_void>> = OnceCell::new();

type LuaScript = Vec<(bool, String)>;
// Scripts waiting to be ran in painttraverse
pub static LUA_SCRIPTS: Lazy<Arc<Mutex<LuaScript>>> =
	Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

pub type Realm = bool;
pub const REALM_MENU: Realm = true;
pub const REALM_CLIENT: Realm = false;
