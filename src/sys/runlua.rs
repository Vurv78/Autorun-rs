#![allow(non_snake_case)]

use crate::sys::{
    funcs::{
        get_lua_error
    },
    statics::*
};
use std::sync::atomic::Ordering;

use rglua::{
    lua_shared::*,
    types::{
        CharBuf,
        LuaState
    }
};

// Runs lua code through loadbufferx. Returns whether it successfully ran.
pub fn runLua(code: &str, verbose: bool) -> bool {
    let state = CURRENT_LUA_STATE.load( Ordering::Relaxed );

    if state == std::ptr::null_mut() {
        eprintln!("Didn't run lua code, make sure the lua state is valid!");
        return false;
    }

    LUAL_LOADBUFFERX.call( state, cstring!(code), std::mem::size_of_val(code), cstring!("@RunString"), cstring!("bt") );

    if let Some(err_type) = get_lua_error( lua_pcall( state, 0, rglua::globals::Lua::MULTRET, 0 ) )  {
        eprintln!("{}: {}", err_type, rstring!( lua_tolstring(state, -1, 0) ) );
        lua_pop!(state, 1);
        return false;
    }
    if verbose { println!("Ran code successfully.") };
    true
}

extern fn log(state: LuaState) -> i32 {
    let s = lua_tostring!(state, 1);
    println!( "LUA LOG: {}", rstring!(s) );
    0
}

// Runs lua, but inside of the sautorun environment.
// sautorun = { NAME: String, CODE: String  }
pub fn runLuaEnv(script: &str, identifier: CharBuf, dumped_script: CharBuf, ip: &str, startup: bool) -> bool {
    let state = CURRENT_LUA_STATE.load( Ordering::Relaxed );

    if state == std::ptr::null_mut() {
        eprintln!("Didn't run lua code, make sure the lua state is valid!");
        return false;
    }

    let loadbufx_hook = &*LUAL_LOADBUFFERX;

    loadbufx_hook.call(state, cstring!(script), std::mem::size_of_val(script), cstring!("@RunString"), cstring!("bt"));

    lua_createtable(state, 0, 0); // Create our custom environment

    lua_createtable(state, 0, 0); // Create the  'sautorun' table

    lua_pushstring( state, identifier );
        lua_setfield( state, -2, cstring!("NAME") );

        lua_pushstring( state, dumped_script );
        lua_setfield( state, -2, cstring!("CODE") );

        lua_pushstring( state, cstring!(ip) );
        lua_setfield( state, -2, cstring!("IP") );

        // If this is running before autorun, set SAUTORUN.STARTUP to true.
        lua_pushboolean( state, startup as i32 );
        lua_setfield( state, -2, cstring!("STARTUP") );

        lua_pushcfunction!( state, log );
        lua_setfield( state, -2, cstring!("log") );

    lua_setfield( state, -2, cstring!(ENV_NAME));

    lua_createtable(state, 0, 0); // Create a metatable to make the env inherit from _G
        lua_pushvalue(state, rglua::globals::Lua::GLOBALSINDEX);
        lua_setfield(state, -2, cstring!("__index"));
    lua_setmetatable(state, -2);

    lua_setfenv(state, -2);

    if let Some(err_type) = get_lua_error( lua_pcall( state, 0, 1, 0 ) )  {
        eprintln!("{}: {}", err_type, rstring!( lua_tolstring(state, -1, 0) ) );
        lua_pop!(state, 1);
        return false;
    }
    true
}