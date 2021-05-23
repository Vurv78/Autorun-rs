#![allow(non_snake_case)]

#[macro_use] extern crate rglua;


// Winapi
use winapi::um::consoleapi::AllocConsole; // Allocate Windows Console for io

use std::{
    io::stdin,
    fs,
    path::Path,
    thread
};

pub mod sys; // Sort of configurable files.
use sys::{
    statics::*,
    runlua::runLua
};

pub mod hooks;

fn handle_input() {
    // Loop forever in this thread, since it is separate from Gmod, and take in user input.
    loop {
        let mut buffer = String::new();
        if let Ok(_) = stdin().read_line(&mut buffer) {
            if buffer.starts_with("lua_run") {
                // Does not work with unicode. Hope you don't somehow get unicode in the console
                let slice = &buffer[8 ..].trim_end();
                runLua( slice, true );
            } else if buffer.starts_with("lua_openscript") {
                let slice = &buffer[15 ..].trim_end();
                match fs::read_to_string( Path::new(slice) ) {
                    Err(why) => {
                        eprintln!("Errored on lua_openscript. [{}]", why);
                    }
                    Ok(contents) => {
                        runLua( &contents, true );
                    }
                }
            } else if buffer.starts_with("help") {
                println!("Commands list:");
                println!("lua_run <code>            | Runs lua code on the currently loaded lua state. Will print if any errors occur.");
                println!("lua_openscript <file_dir> | Runs a lua script located at file_dir, this dir being a full directory, not relative or anything.");
                println!("help                      | Prints this out.");
            }
        }
    }
}

fn entry_point() {
    assert!( unsafe { AllocConsole() } == 1 ,"Couldn't allocate console.");
    println!("<--> Autorun-rs <-->");
    println!("Type [help] for the list of commands.");

    &*LUAL_LOADBUFFERX; // Initialize loadbufferx hook. Might not be necessary.

    handle_input();
}

// Windows Only. I'm not going to half-ass cross-operating system support.
#[no_mangle]
pub extern "stdcall" fn DllMain(_: *const u8, reason: u32, _: *const u8) -> u32 {
    match reason {
        1 => {
            // DLL_PROCESS_ATTACH
            thread::spawn(entry_point);
        }
        0 => {
            // DLL_PROCESS_DETACH
            // Detour cleanups
            #[allow(unused_must_use)]
            unsafe {
                LUAL_LOADBUFFERX.disable();
                if let Some(hook) = JOIN_SERVER.get() {
                    hook.disable();
                }
            };
        }
        _ => ()
    }
    1
}