/// Most of this should be reworked later.
use std::sync::{Arc, Mutex};

use once_cell::sync::Lazy;

use autorun_shared::Realm;

type LuaScript = Vec<(Realm, String)>;
// Scripts waiting to be ran in painttraverse
pub static LUA_SCRIPTS: Lazy<Arc<Mutex<LuaScript>>> =
	Lazy::new(|| Arc::new(Mutex::new(Vec::new())));