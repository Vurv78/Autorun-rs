

#![allow(non_snake_case)]

// Winapi
use winapi::shared::minwindef as WinStructs; // Types from the windows api
use WinStructs::HINSTANCE;
use winapi::um::consoleapi::AllocConsole; // Allocate Windows Console for io

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::io::{self, prelude::* }; // Read user input for commands

// detours-rs
use detour::GenericDetour;

// dlopen-rs
#[macro_use] extern crate dlopen_derive;
use dlopen::wrapper::{Container, WrapperApi};

use lazy_static::lazy_static; // lazy_static! macro

// Types
use std::ffi::{CStr, CString};
use libc::c_void as CVoid;
type CChar = i8;
type LuaState = *mut CVoid;
type CharBuf = *const CChar; // *const char
type SizeT = usize;
type CInt = i32;
type GlobalDetour<T> = Option<GenericDetour<T>>;
type LuaCFunction = extern "C" fn(LuaState) -> CInt;

static mut JOIN_SERVER: GlobalDetour<LuaCFunction> = None;
static mut LUAL_LOADBUFFERX: GlobalDetour< extern fn(LuaState, CharBuf, SizeT, CharBuf, CharBuf) -> CInt > = None;

static mut CURRENT_LUA_STATE: Option<LuaState> = None;
static mut CURRENT_SERVER_IP: Option<&'static str> = None;

// Functions from lua_shared.dll. Will search for the dll relative to the GarrysMod folder,
// Which rust finds it by looking at the program the dll is attached to (Gmod.)
#[derive(WrapperApi)]
struct LuaShared {
    luaL_loadbufferx: extern fn(state: LuaState, code: CharBuf, size: SizeT, id: CharBuf, mode: CharBuf) -> CInt,
    luaL_loadbuffer: extern fn(state: LuaState, code: CharBuf, size: SizeT, id: CharBuf) -> CInt,
    luaL_loadstring: extern fn(state: LuaState, code: CharBuf) -> CInt,
    lua_pcall: extern fn(state: LuaState, nargs: CInt, nresults: CInt, msgh: CInt) -> CInt,
    lua_tolstring: extern fn(state: LuaState, ind: CInt, size: SizeT) -> CharBuf,
    lua_settop: extern fn(state: LuaState, ind: CInt),
    lua_getfield: extern fn(state: LuaState, idx: CInt, key: CharBuf) -> CInt,
    lua_tocfunction: extern fn(state: LuaState, idx: CInt) -> LuaCFunction,
    CreateInterface: extern fn(name: CharBuf, ret_code: CInt) -> *mut CVoid
}

lazy_static! {
    static ref GMOD_PATH: PathBuf = std::env::current_dir().unwrap(); // D:\SteamLibrary\steamapps\common\GarrysMod for example.
    static ref BIN_PATH: PathBuf = {
        let bin = Path::new(&*GMOD_PATH).join("bin");
        if cfg!( target_arch = "x86_64" ) {
            return bin.join("win64");
        }else{
            return bin;
        }
    };
    static ref LUA_SHARED_PATH: PathBuf = Path::new( &*BIN_PATH ).join("lua_shared.dll");

    static ref LUA_SHARED_LIB: Container<LuaShared> = {
        let dll_path = &*LUA_SHARED_PATH;
        match unsafe {Container::<LuaShared>::load(dll_path)} {
            Ok(lib) => lib,
            Err(why) => panic!("Path DLL tried to load: {}, Error Reason: {}. Report this on github.", dll_path.display(), why)
        }
    };
}

// #define lua_getglobal(L,s)      lua_getfield(L, LUA_GLOBALSINDEX, (s))

// lua_pop implementation like the C macro.
// TODO: Make this a rust macro, alongside lua_getglobal.
fn lua_pop(ls: &Container<LuaShared>, state: LuaState, ind: CInt) {
    ls.lua_settop(state, -(ind)-1);
}

// Recursively creates folders based off of a directory from your HOME dir + the lua path made from the currently running file.
// &str garry_dir = Not necessarily a directory, can be anything, but this is the id returned by loadbuffer, loadstring, etc. Ex: "lua/init/bruh.lua"
// &str server_ip = The ip of the server. This will be used to create the folder structure of HOME/sautorun-rs/lua_dumps/IP/...
// Returns Option<File> that was created at the final dir.
fn get_autorun_file(garry_dir: &str, server_ip: &str) -> Option<File> {
    if garry_dir.len() > 500 { return None }; // If the server wants to try and attack your fs.
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

    let home = match dirs::home_dir() {
        Some(path) => {
            path.join("sautorun-rs").join("lua_dumps").join(server_ip.replace(":",".")).join(&lua_run_path)
        },
        None => {
            println!("Couldn't get home directory, for whatever reason.");
            return None; // Abort get_autorun_file
        }
    };

    match home.parent() {
        Some(dirs) => {
            match fs::create_dir_all(dirs) {
                Err(why) => {
                    println!("Couldn't create sautorun-rs directories. [{}]", why);
                    dbg!(dirs);
                    None
                }
                Ok(_) => {
                    match File::create(home) {
                        Ok(file) => Some(file),
                        Err(why) => {
                            println!("Couldn't create sautorun-rs file. [{}]", why);
                            None
                        }
                    }
                }
            }
        }
        None => None
    }
}

extern fn h_loadbufferx(state: LuaState, code: CharBuf, size: SizeT, identifier: CharBuf, mode: CharBuf) -> CInt {
    let raw_path = &rust_str(identifier)[1 ..];
    let server_ip = unsafe {
        CURRENT_LUA_STATE = Some(state); // Hijack the lua state.
        CURRENT_SERVER_IP.unwrap_or("unknown_ip")
    };
    if let Some(mut file) = get_autorun_file(raw_path, server_ip) {
        if let Err(why) = file.write_all( rust_str(code).as_bytes() ) {
            println!("Couldn't write to file made from lua path [{}]. {}", raw_path, why);
        }
    }
    if let Some(hook) = unsafe{ &LUAL_LOADBUFFERX } {
        return hook.call( state, code, size, identifier, mode ); // Call the original function and return the value.
    }
    println!("Failed to get LUAL_LOADBUFFERX hook");
    0
}

extern fn h_join_server(state: LuaState) -> CInt {
    unsafe {
        let lua_shared = &*LUA_SHARED_LIB;
        let ip = rust_str(lua_shared.lua_tolstring(state, 1, 0));
        println!("Joining Server with IP {}!", ip);
        CURRENT_SERVER_IP = Some(ip);
        if let Some(hook) = &JOIN_SERVER {
            return hook.call(state);
        }
    }
    println!("Failed to get JOIN_SERVER hook.");
    0
}

// Turns a rust &str into a CString
// Could alternatively use std::mem::forget to return *const i8 instead.
fn c_string(s: &str) -> CString {
    match CString::new(s) {
        Ok(cstring) => cstring,
        Err(why) => {
            panic!("NulError. {}", why);
        }
    }
}

// rust_str(c_string("test").as_ptr()) -> "test"
fn rust_str<'a>(s: CharBuf) -> &'a str {
    let cstr = unsafe{ CStr::from_ptr(s) };
    cstr.to_str().unwrap_or("")
}

fn get_lua_error<'a>(err: i32) -> Option<&'a str> {
    match err {
        0 => None, // Ok
        1 => Some("Yield"), // Yield
        2 => Some("Error at runtime"),
        3 => Some("Syntax error"),
        4 => Some("Ran out of memory"),
        5 => Some("Errored during garbage collection"),
        6 => Some("Errored inside error message handler"),
        _ => unreachable!()
    }
}

// When this DLL is attached
fn detour_funcs() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let lua_shared = &*LUA_SHARED_LIB;
        // Setting the static vars

        let loadbufxh = GenericDetour::new( lua_shared.luaL_loadbufferx, h_loadbufferx )?;
        loadbufxh.enable()?;
        LUAL_LOADBUFFERX = Some(loadbufxh);
    };
    Ok(())
}

fn runLua(code: &str) {
    if let Some(state) = unsafe{ CURRENT_LUA_STATE } {
        if let Some(loadbufx_hook) = unsafe { &LUAL_LOADBUFFERX } {
            let lua_shared = &*LUA_SHARED_LIB;
            loadbufx_hook.call( state, c_string(code).as_ptr(), std::mem::size_of_val(code), c_string("@RunString").as_ptr(), c_string("bt").as_ptr() );
            let result = lua_shared.lua_pcall( state, 0, -1, 0 );
            if let Some(err_type) = get_lua_error(result)  {
                // TODO: Running while true do end causes a stack leak here.
                println!("{}: {}", err_type, rust_str(lua_shared.lua_tolstring(state, -1, 0)) );
                lua_pop(lua_shared, state, 1);
            } else {
                println!("Code ran successfully.");
            }
        }
    }
}

fn handle_input() {
    // Loop forever in this thread, since it is separate from Gmod, and take in user input.
    loop {
        let mut buffer = String::new();
        if let Ok(_) = io::stdin().read_line(&mut buffer) {
            if buffer.starts_with("lua_run") {
                // Does not work with unicode. Hope you don't somehow get unicode in the console
                let slice = &buffer[8 ..].trim_end();
                runLua(slice);
            } else if buffer.starts_with("lua_openscript") {
                let slice = &buffer[15 ..].trim_end();
                match fs::read_to_string( Path::new(slice) ) {
                    Err(why) => {
                        println!("Errored on lua_openscript. [{}]", why);
                    }
                    Ok(contents) => {
                        runLua(&contents);
                    }
                }
            } else if buffer.starts_with("init_file_steal") {
                // Run this in the menu state. Hopefully will automate this with CreateInterface or something.
                if let Some(state) = unsafe{ CURRENT_LUA_STATE } {
                    let ls = &*LUA_SHARED_LIB;
                    ls.lua_getfield(state, -10002, c_string("JoinServer").as_ptr() ); // lua_getglobal
                    unsafe {
                        match GenericDetour::new( ls.lua_tocfunction(state, -1), h_join_server ) {
                            Ok(hook) => {
                                hook.enable().expect("Couldn't enable JoinServer hook");
                                JOIN_SERVER = Some(hook);
                                println!("Successfully hooked JoinServer.");
                            }
                            Err(why) => {
                                println!("Couldn't hook JoinServer. {}", why);
                            }
                        }
                    }
                    lua_pop(ls, state, 1);
                }else {
                    println!("Run this command when you've caused menu state lua to run, hover over a ui button or something!");
                }
            } else if buffer.starts_with("help") {
                println!("Commands list:");
                println!("lua_run <code>            | Runs lua code on the currently loaded lua state. Will print if any errors occur.");
                println!("lua_openscript <file_dir> | Runs a lua script located at file_dir, this dir being a full directory, not relative or anything.");
                println!("init_file_steal           | Hooks JoinServer so that we can get the IP of servers you join. Do this before expecting any files to be dumped. (Will hopefully be automated at some point, only needs to be called once in the menu tho.)");
                println!("help                      | Prints this out.");
            }
        }
    }
}

fn entry_point() {
    assert!( unsafe { AllocConsole() }==1 ,"Couldn't allocate console.");
    println!("<--> Autorun-rs <-->");
    println!("Type [help] for the list of commands.");
    if let Err(why) = detour_funcs() {
        println!("Failed to detour functions. {}", why);
        loop {} // Lock the main thread so you actually see the error, panic would just crash gmod.
    } else {
        println!("Successfully detoured functions.");
    }
    handle_input();
}

// Windows Only. I'm not going to half-ass cross-operating system support.
#[no_mangle]
pub extern "stdcall" fn DllMain(_: HINSTANCE, reason: u32, _: *mut CVoid) -> WinStructs::BOOL {
    match reason {
        1 => {
            // DLL_PROCESS_ATTACH
            std::thread::spawn(entry_point);
        }
        0 => {
            // DLL_PROCESS_DETACH
            // Todo
        }
        _ => ()
    }
    WinStructs::TRUE
}

// cargo build --release // --target=i686-pc-windows-msvc