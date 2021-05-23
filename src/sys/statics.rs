use std::{
    path::PathBuf,
    sync::atomic::{
        AtomicBool,
        AtomicPtr,
    }
};

use atomic::Atomic;

use once_cell::sync::{Lazy, OnceCell};

use rglua::{
    lua_shared::luaL_loadbufferx,
    types::{
        LuaState,
        LuaCFunction,
        CharBuf,
        CInt,
        SizeT,
        CVoid
    }
};

use detour::GenericDetour; // detours-rs

// ---------------- Configs ---------------- //
pub static HOME_DIR: Lazy<PathBuf> = Lazy::new(|| dirs::home_dir().expect("Couldn't get your home directory!") );
pub static SAUTORUN_DIR: Lazy<PathBuf> = Lazy::new(|| HOME_DIR.join("sautorun-rs") );


// This location is run right before autorun.
pub static AUTORUN_SCRIPT_PATH: Lazy<PathBuf> = Lazy::new(|| (*SAUTORUN_DIR).join("autorun.lua") );

// Basically ROC, whenever a lua script is ran, run this and pass the code. If it returns true or nil, run the code, else don't
pub static HOOK_SCRIPT_PATH: Lazy<PathBuf> = Lazy::new(|| (*SAUTORUN_DIR).join("hook.lua") );

pub static ENV_NAME: &'static str = "sautorun"; // Name of the env table / index in _G

// ---------------- Configs ---------------- //

// No more static mut! 🥳

// AtomicPtr automatically attaches *mut to the type given. That's why we give CVoid instead of LuaState, because we'll end up with *mut CVoid aka LuaState
pub static CURRENT_LUA_STATE: AtomicPtr<CVoid>                     = AtomicPtr::new( std::ptr::null_mut() ); // Not using AtomicPtr::default() because it isn't a static function
pub static CURRENT_SERVER_IP: Atomic<&'static str>                 = Atomic::new("unknown_ip"); // Using Atomic crate because there is no official way to get an atomic str / string.

pub static HAS_AUTORAN: AtomicBool                                 = AtomicBool::new(false); // Whether an autorun script has been run and detected already.
pub static JOIN_SERVER: OnceCell< GenericDetour < LuaCFunction > > = OnceCell::new();

pub static LUAL_LOADBUFFERX: Lazy< GenericDetour< extern fn(LuaState, CharBuf, SizeT, CharBuf, CharBuf) -> CInt > > = Lazy::new(|| {
    unsafe {
        match GenericDetour::new( *luaL_loadbufferx, crate::hooks::loadbufferx ) {
            Ok(b) => {
                b.enable().expect("Couldn't enable LOADBUFFERX hook.");
                b
            },
            Err(why) => panic!("Errored when hooking LOADBUFFERX. Why: {}. Report this on github.", why)
        }
    }
});
