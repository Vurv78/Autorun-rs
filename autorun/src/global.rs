/// Most of this should be reworked later.
use std::os::raw::c_void;

use std::sync::atomic::{AtomicBool, AtomicPtr};
use std::sync::{Arc, Mutex};

use atomic::Atomic;

use once_cell::sync::{Lazy, OnceCell};

// AtomicPtr automatically attaches *mut to the type given. That's why we give CVoid instead of LuaState, because we'll end up with *mut CVoid aka LuaState
pub static CLIENT_STATE: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut()); // Not using AtomicPtr::default() because it isn't a static function
pub static SERVER_IP: Atomic<&str> = Atomic::new("0.0.0.0"); // Using Atomic crate because there is no official way to get an atomic str / string.

pub static HAS_AUTORAN: AtomicBool = AtomicBool::new(false); // Whether an autorun script has been run and detected already.
pub static MENU_STATE: OnceCell<AtomicPtr<c_void>> = OnceCell::new();

type LuaScript = Vec<(bool, String)>;
// Scripts waiting to be ran in painttraverse
pub static LUA_SCRIPTS: Lazy<Arc<Mutex<LuaScript>>> =
	Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

pub static LOGGING_ENABLED: AtomicBool = AtomicBool::new(true);
pub static FILESTEAL_ENABLED: AtomicBool = AtomicBool::new(true);