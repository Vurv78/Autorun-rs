# Autorun-rs
[![Release Shield](https://img.shields.io/github/v/release/Vurv78/Autorun-rs)](https://github.com/Vurv78/Autorun-rs/releases/latest)
[![License](https://img.shields.io/github/license/Vurv78/Autorun-rs?color=red)](https://opensource.org/licenses/Apache-2.0)
![CI](https://github.com/Vurv78/Autorun-rs/workflows/Build/badge.svg) [![github/Vurv78](https://img.shields.io/discord/824727565948157963?color=7289DA&label=chat&logo=discord)](https://discord.gg/epJFC6cNsw)

Garrysmod Lua Dumper / Runner, written in Rust.
The file structure starts at ``C:\Users\<User>\sautorun-rs\``

### Features
* Dumping all lua scripts to ``C:\Users\<User>\sautorun-rs\lua_dumps\<ServerIP>\..``
* Runtime lua loading through lua_run and lua_openscript in an external console
* Supports x86 and x64 bit
* Scripthook, stop & run scripts before anything runs on you, gives information & functions to assist in a safe separate lua fenv

### Usage
Get an injector (Make sure it's compatible to inject 32/64 bit code depending on your use).  
Use one of the build_win batchfiles to build the output dll, then inject that into garrysmod at the main menu.  

### Notes
* For now this is only supporting Windows.  
  In the future this may be able to run on Linux/OSX but if that does happen I won't be able to actively support it.

### Examples

sautorun-rs/hook.lua
```lua
local script = sautorun.CODE
if script:find("while true do end") then
  sautorun.log("Found a naughty script!")
  return true -- Exit from here & don't run the script
end
```

sautorun-rs/autorun.lua
```lua
sautorun.log( "Connected to server " .. sautorun.IP )
```

### TODOs
* Actual GUI