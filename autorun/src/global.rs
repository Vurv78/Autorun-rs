#[cfg(feature = "runner")]
#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
use std::sync::{Arc, Mutex};

#[cfg(feature = "runner")]
#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
use once_cell::sync::Lazy;

#[cfg(feature = "runner")]
#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
type LuaScript = Vec<(autorun_shared::Realm, String)>;

// Scripts waiting to be ran in painttraverse
#[cfg(feature = "runner")]
#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
pub static LUA_SCRIPTS: Lazy<Arc<Mutex<LuaScript>>> =
	Lazy::new(|| Arc::new(Mutex::new(Vec::new())));