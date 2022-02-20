/// Most of this should be reworked later.
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use once_cell::sync::Lazy;

use autorun_shared::Realm;

pub static HAS_AUTORAN: AtomicBool = AtomicBool::new(false); // Whether an autorun script has been run and detected already.

type LuaScript = Vec<(Realm, String)>;
// Scripts waiting to be ran in painttraverse
pub static LUA_SCRIPTS: Lazy<Arc<Mutex<LuaScript>>> =
	Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

pub static LOGGING_ENABLED: AtomicBool = AtomicBool::new(true);
pub static FILESTEAL_ENABLED: AtomicBool = AtomicBool::new(true);
