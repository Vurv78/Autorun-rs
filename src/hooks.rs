use std::{
    io::prelude::*,
    sync::atomic::Ordering,
    ffi::CStr,
    fs
};

use crate::sys::{
    funcs::getAutorunHandle,
    statics::*,
    runlua::runLuaEnv
};

use rglua::{
    lua_shared::*,
    types::*
};

#[macro_export]
macro_rules! get_cstring {
    ($cstring:expr) => {
        {
            #[allow(unused_unsafe)]
            let cstr = unsafe{ CStr::from_ptr($cstring) };
            cstr.to_owned()
        }
    }
}

pub extern fn loadbufferx(state: LuaState, code: CharBuf, size: SizeT, identifier: CharBuf, mode: CharBuf) -> CInt {
    if state != std::ptr::null_mut() {
        CURRENT_LUA_STATE.store(state, Ordering::Relaxed);
    }

    // If JoinServer hasn't been hooked, hook it.
    let _ = JOIN_SERVER.get_or_try_init(|| {
        lua_getglobal!( state, cstring!("JoinServer") );
        let hook = match unsafe { detour::GenericDetour::new( lua_tocfunction(state, -1), crate::hooks::joinserver ) } {
            Ok(hook) => {
                unsafe {
                    hook.enable().expect("Couldn't enable JoinServer hook");
                }

                Ok(hook)
            }
            Err(why) => {
                eprintln!("Couldn't hook JoinServer. {}", why);
                return Err(());
            }
        };
        lua_pop!(state, 1);
        hook
    });

    // Todo: Check if you're in menu state (Not by checking MENU_DLL because that can be modified by lua) and if so, don't dump files.
    // Dump the file to sautorun-rs/lua_dumps/IP/...
    let raw_path = &rstring!(identifier)[1 ..]; // Remove the @ from the beginning of the path.
    let server_ip = CURRENT_SERVER_IP.load( Ordering::Relaxed );

    let loadbuffer_h = &*LUAL_LOADBUFFERX;

    let mut autoran = false;
    let mut do_run = true;
    if raw_path == "lua/includes/init.lua" {
        if let Ok(_) = HAS_AUTORAN.compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed) {
            // This will only run once when HAS_AUTORAN is false, setting it to true.
            // Will be reset by JoinServer.
            if let Ok(script) = fs::read_to_string(&*AUTORUN_SCRIPT_PATH) {
                // Try to run here
                runLuaEnv(&script, identifier, code, server_ip, true);
                autoran = true;
            } else {
                eprintln!( "Couldn't read your autorun script file at {}/{}", SAUTORUN_DIR.display(), AUTORUN_SCRIPT_PATH.display() );
            }
        }
    }

    if !autoran {
        if let Ok(script) = fs::read_to_string(&*HOOK_SCRIPT_PATH) {
            if runLuaEnv(&script, identifier, code, server_ip, false) {
                // If you return ``true`` in your sautorun/hook.lua file, then don't run the sautorun.CODE that is about to run.
                if lua_type(state, 1) == rglua::globals::Lua::Type::Bool as i32 {
                    do_run = lua_toboolean(state, 1) == 0;
                    lua_pop!(state, 1);
                }
            }
        }

    }

    if let Some(mut file) = getAutorunHandle(raw_path, server_ip) {
        if let Err(why) = file.write_all( get_cstring!(code).as_bytes() ) {
            eprintln!("Couldn't write to file made from lua path [{}]. {}", raw_path, why);
        }
    }

    if do_run {
        return loadbuffer_h.call( state, code, size, identifier, mode ); // Call the original function and return the value.
    }
    0
}

// Since the first lua state will always be the menu state, just keep a variable for whether joinserver has been hooked or not,
// If not, then hook it.
pub extern fn joinserver(state: LuaState) -> CInt {
    let ip = rstring!( lua_tolstring(state, 1, 0) );
    println!("Joining Server with IP {}!", ip);

    CURRENT_SERVER_IP.store(ip, Ordering::Relaxed); // Set the IP so we know where to write files in loadbufferx.
    HAS_AUTORAN.store(false, Ordering::Relaxed);
    if let Some(hook) = JOIN_SERVER.get() {
        // We could retrieve the hook from our global variables
        hook.call(state);
    } else {
        eprintln!("Failed to get JOIN_SERVER hook from global state");
    }
    0
}