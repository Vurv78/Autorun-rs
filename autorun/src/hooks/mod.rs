use std::ffi::CStr;
use std::sync::{
	atomic::{AtomicU64, Ordering},
	MutexGuard,
};

use crate::{configs::SETTINGS, logging, lua};

use logging::*;
use rglua::interface;
use rglua::prelude::*;

mod dumper;
pub mod lazy;
mod scripthook;
use lazy::lazy_detour;

// Make our own static detours because detours.rs is lame and locked theirs behind nightly. :)
lazy_detour! {
	pub static LUAL_LOADBUFFERX_H: extern "C" fn(LuaState, *const i8, SizeT, *const i8, *const i8) -> i32 = (
		{
			*LUA_SHARED_RAW.get::<extern "C" fn(LuaState, LuaString, SizeT, LuaString, LuaString) -> i32>(b"luaL_loadbufferx")
				.expect("Failed to get luaL_loadbufferx")
		},
		loadbufferx_h
	);

	#[cfg(feature = "runner")]
	#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
	pub static PAINT_TRAVERSE_H: PaintTraverseFn = (
		{
			let vgui = iface!(Panel).expect("Failed to get Panel interface");
			std::mem::transmute::<_, PaintTraverseFn>(
				(vgui.vtable as *mut *mut c_void)
					.offset(41)
					.read(),
			)
		},
		paint_traverse_h
	);
}

#[cfg(feature = "runner")]
#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
use rglua::interface::Panel;

#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
type PaintTraverseFn = extern "fastcall" fn(&'static Panel, usize, bool, bool);

static CONNECTED: AtomicU64 = AtomicU64::new(99999);

pub struct DispatchParams<'a> {
	ip: LuaString,

	code: LuaString,
	code_len: usize,

	identifier: LuaString,

	startup: bool,
	path: &'a str,
	#[allow(unused)]
	engine: &'a mut interface::EngineClient,
	net: &'a mut interface::NetChannelInfo,
}

impl<'a> DispatchParams<'a> {
	pub fn set_code(&mut self, code: LuaString, code_len: usize) {
		self.code = code;
		self.code_len = code_len;
	}

	pub fn get_code(&self) -> (LuaString, usize) {
		(self.code, self.code_len)
	}
}

extern "C" fn loadbufferx_h(
	l: LuaState,
	mut code: LuaString,
	mut code_len: SizeT,
	identifier: LuaString,
	mode: LuaString,
) -> i32 {
	if let Ok(mut engine) = iface!(EngineClient) {
		let do_run;
		if engine.IsConnected() {
			let net = engine.GetNetChannelInfo();

			if let Some(net) = unsafe { net.as_mut() } {
				let ip = net.GetAddress();
				let mut startup = false;

				// TODO: It'd be great to hook net connections instead of doing this.
				// However, this works fine for now.
				let curtime = net.GetTimeConnected() as u64;
				if curtime < CONNECTED.load(Ordering::Relaxed) {
					debug!("Curtime is less than last time connected, assuming startup");
					startup = true;

					if let Err(why) = close_dylibs() {
						debug!("Failed to close dynamic libs: {why}");
					}
				}

				// Awful
				CONNECTED.store(curtime, Ordering::Relaxed);

				let path = unsafe { CStr::from_ptr(identifier) };
				let path = &path.to_string_lossy()[1..]; // Remove the @ from the beginning of the path

				// There's way too many params here
				let mut params = DispatchParams {
					ip,

					code,
					code_len,

					identifier,
					startup,
					path,

					engine: &mut engine,
					net,
				};

				do_run = dispatch(l, &mut params);
				if do_run {
					(code, code_len) = params.get_code();
				} else {
					return 0;
				}
			}
		}
	}

	unsafe { LUAL_LOADBUFFERX_H.call(l, code, code_len, identifier, mode) }
}

pub fn dispatch(l: LuaState, params: &mut DispatchParams) -> bool {
	let mut do_run = true;

	scripthook::execute(l, params, &mut do_run);
	if SETTINGS.filesteal.enabled {
		dumper::dump(params);
	}

	do_run
}

#[cfg(feature = "runner")]
#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
extern "fastcall" fn paint_traverse_h(
	this: &'static Panel,
	panel_id: usize,
	force_repaint: bool,
	force_allow: bool,
) {
	unsafe {
		PAINT_TRAVERSE_H.call(this, panel_id, force_repaint, force_allow);
	}

	if let Ok(ref mut queue) = lua::SCRIPT_QUEUE.try_lock() {
		if !queue.is_empty() {
			let (realm, script) = queue.remove(0);

			match lua::get_state(realm) {
				Ok(state) => match lua::dostring(state, &script) {
					Err(why) => error!("{why}"),
					Ok(_) => info!("Script of len #{} ran successfully.", script.len()),
				},
				Err(why) => error!("{why}"),
			}
		}
	}
}

#[derive(Debug, thiserror::Error)]
enum CloseLibs {
	#[error("Failed to acquire mutex (Report this on github)")]
	Mutex(#[from] std::sync::PoisonError<MutexGuard<'static, Vec<libloading::Library>>>),
}

/// Closes all previously loaded dylibs from Autorun.requirebin
fn close_dylibs() -> Result<(), CloseLibs> {
	let mut libs = lua::LOADED_LIBS.lock()?;

	for lib in libs.drain(..) {
		let _ = lib.close();
	}

	Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum HookingError {
	#[error("Failed to hook function: {0}")]
	Detour(#[from] detour::Error),

	#[error("Failed to get interface")]
	Interface(#[from] rglua::interface::Error),
}

pub fn init() -> Result<(), HookingError> {
	use once_cell::sync::Lazy;

	Lazy::force(&LUAL_LOADBUFFERX_H);

	#[cfg(feature = "runner")]
	#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
	Lazy::force(&PAINT_TRAVERSE_H);

	dumper::start_queue();

	Ok(())
}

pub fn cleanup() -> Result<(), detour::Error> {
	unsafe {
		LUAL_LOADBUFFERX_H.disable()?;

		#[cfg(feature = "runner")]
		#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
		PAINT_TRAVERSE_H.disable()?;
	}

	if let Err(why) = close_dylibs() {
		debug!("Failed to close dynamic libs: {why}");
	}

	Ok(())
}
