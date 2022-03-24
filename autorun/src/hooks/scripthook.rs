use crate::{
	configs::{self, SETTINGS},
	lua::{self, AutorunEnv},
	plugins,
};
use fs_err as fs;
use rglua::prelude::*;

use super::DispatchParams;

pub fn execute(l: LuaState, params: &mut DispatchParams, do_run: &mut bool) {
	if params.startup {
		// autorun.lua
		let env = AutorunEnv {
			is_autorun_file: true,
			startup: params.startup,

			identifier: params.identifier,
			code: params.code,
			code_len: params.code_len,

			ip: params.ip,
			plugin: None,
		};

		if let Err(why) = plugins::call_autorun(l, &env) {
			error!("Failed to call plugins (autorun): {why}");
		}
		// This will only run once when HAS_AUTORAN is false, setting it to true.
		// Will be reset by JoinServer.
		let ar_path = configs::path(configs::AUTORUN_PATH);
		trace!("Running autorun script at {}", ar_path.display());

		if let Ok(script) = fs::read_to_string(&ar_path) {
			// Try to run here
			if let Err(why) = lua::run_env(l, &script, &env) {
				error!("{why}");
			}
		} else {
			debug!(
				"Couldn't read your autorun script file at [{}]",
				ar_path.display()
			);
		}
	}

	{
		// hook.lua
		let env = AutorunEnv {
			is_autorun_file: false,
			startup: params.startup,

			identifier: params.identifier,
			code: params.code,
			code_len: params.code_len,

			ip: params.ip,
			plugin: None,
		};

		if SETTINGS.plugins.enabled {
			if let Err(why) = plugins::call_hook(l, &env) {
				error!("Failed to call plugins (hook): {why}");
			}
		}

		if let Ok(script) = fs::read_to_string(configs::path(configs::HOOK_PATH)) {
			match lua::run_env(l, &script, &env) {
				Ok(top) => {
					// If you return ``true`` in your sautorun/hook.lua file, then don't run the sautorun.CODE that is about to run.
					match lua_type(l, top + 1) {
						rglua::lua::TBOOLEAN => {
							*do_run = lua_toboolean(l, top + 1) == 0;
						}
						rglua::lua::TSTRING => {
							// lua_tolstring sets len to new length automatically.
							let nul_str = lua_tolstring(l, top + 1, &mut params.code_len);
							params.code = nul_str;
						}
						_ => (),
					}
					lua_settop(l, top);
				}
				Err(_why) => (),
			}
		}
	}
}
