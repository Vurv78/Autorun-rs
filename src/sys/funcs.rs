use std::{
    path::PathBuf,
    fs::{self, File},
    sync::atomic::Ordering
};


use rglua::types::LuaState;

use crate::sys::{
    statics::*
};

// Recursively creates folders based off of a directory from your HOME dir + the lua path made from the currently running file.
// &str garry_dir = Not necessarily a directory, can be anything, but this is the id returned by loadbuffer, loadstring, etc. Ex: "lua/init/bruh.lua"
// &str server_ip = The ip of the server. This will be used to create the folder structure of HOME/sautorun-rs/lua_dumps/IP/...
// Returns Option<File> that was created at the final dir.
pub fn getAutorunHandle(garry_dir: &str, server_ip: &str) -> Option<File> {
    if garry_dir.len() > 500 { return None };
    let mut lua_run_path = PathBuf::from(garry_dir);

    let extension = match lua_run_path.extension() {
        Some(ext) => {
            match ext.to_str() {
                Some(ext) if ext=="lua" => "lua", // Using guards check if the extension is lua, else it will fall under _.
                _ => "txt"
            }
        }
        None => "txt"
    };
    lua_run_path.set_extension(extension);

    let file_loc = &*SAUTORUN_DIR
        .join("lua_dumps")
        .join(server_ip.replace(":","."))
        .join(&lua_run_path);

    match file_loc.parent() {
        Some(dirs) => {
            match fs::create_dir_all(dirs) {
                Err(why) => {
                    eprintln!("Couldn't create sautorun-rs directories. [{}]", why);
                    dbg!(dirs);
                    None
                }
                Ok(_) => {
                    match File::create(file_loc) {
                        Ok(file) => Some(file),
                        Err(why) => {
                            eprintln!("Couldn't create sautorun-rs file. [{}]", why);
                            None
                        }
                    }
                }
            }
        }
        None => None
    }
}

// Returns the name of the error from a Lua C API returned error number.
pub fn get_lua_error<'a>(err: i32) -> Option<&'a str> {
    match err {
        0 => None, // Ok
        1 => None, // Coroutine yield
        2 => Some("Error at runtime"),
        3 => Some("Syntax error"),
        4 => Some("Ran out of memory"),
        5 => Some("Errored during garbage collection"),
        6 => Some("Errored inside error message handler"),
        _ => Some("Unknown")
    }
}

// Creating this function for the future where accessing the lua state doesn't directly need an unsafe block.
pub fn getLuaState() -> LuaState {
    CURRENT_LUA_STATE.load( Ordering::Relaxed )
}

pub fn setLuaState(state: LuaState) {
    CURRENT_LUA_STATE.store( state, Ordering::Relaxed );
}