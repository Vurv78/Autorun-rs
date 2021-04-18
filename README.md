## Autorun-rs [![Release Shield](https://img.shields.io/github/v/release/Vurv78/Autorun-rs)](https://github.com/Vurv78/Autorun-rs/releases/latest) [![License](https://img.shields.io/github/license/Vurv78/Autorun-rs?color=red)](https://opensource.org/licenses/Apache-2.0) ![CI](https://github.com/Vurv78/Autorun-rs/workflows/Build/badge.svg) [![github/Vurv78](https://discordapp.com/api/guilds/824727565948157963/widget.png)](https://discord.gg/epJFC6cNsw)

Garrysmod Lua Dumper / Runner, written in Rust.

Like my other repo, https://github.com/Vurv78/Autorun, but written in Rust.  
Also, this has more features and is safer.

The file structure starts at C:\Users\User\sautorun-rs\.

### Features
* Lua dumping through hooked loadbufferx at C:\Users\User\sautorun-rs\lua_dumps\ServerIP\... (Use init_file_steal)
* Lua loading through lua_run and lua_openscript
* Could load lua scripts before autorun.
  * Not supported out of the box, could add it yourself relatively easily
* Separate AllocConsole that allows for running commands (See the help command)
* Works for both x64 and x86 windows. (Tested on Chromium / x86-64 branch)

### Usage
Get an injector (Make sure it's compatible to inject 32/64 bit code depending on your use).  
Use one of the build_win batchfiles to build the output dll, then inject that into garrysmod at the main menu.  
See the Notes for more info.

### TODOs
* Automating JoinServer hook
* Making a lua script in sautorun run before autorun out of the box.

### Notes
This is Windows ONLY. Autorun might've had a chance at being multi-platform but this is absolutely not going to work with OSX/Linux.  
You need to call init_file_steal in your garrysmod main menu in the Autorun terminal, so that files will be dumped.  
