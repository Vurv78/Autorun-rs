use crate::{
	configs::SETTINGS,
	fs::{self as afs, AUTORUN_PATH, HOOK_PATH},
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
		let full_path = afs::in_autorun(AUTORUN_PATH);
		if let Ok(script) = fs::read_to_string(&full_path) {
			if let Err(why) = lua::run_env(l, &script, AUTORUN_PATH, &env) {
				error!("{why}");
			}
		} else {
			debug!(
				"Couldn't read your autorun script file at [{}]",
				full_path.display()
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
			match plugins::call_hook(l, &env, do_run) {
				Err(why) => {
					error!("Failed to call plugins (hook): {why}");
				}
				Ok(Some((code, len))) => {
					params.set_code(code, len);
				}
				Ok(_) => (),
			}
		}

		if let Ok(script) = afs::read_to_string(HOOK_PATH) {
			match lua::run_env(l, &script, HOOK_PATH, &env) {
				Ok(top) => {
					// If you return ``true`` in your hook.lua file, then don't run the Autorun.CODE that is about to run.
					match lua_type(l, top + 1) {
						rglua::lua::TBOOLEAN => {
							if lua_toboolean(l, top + 1) != 0 {
								*do_run = false;
							}
						}
						rglua::lua::TSTRING => {
							// lua_tolstring sets len to new length automatically.
							let mut len: usize = 0;
							let newcode = lua_tolstring(l, top + 1, &mut len);
							params.set_code(newcode, len);
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
