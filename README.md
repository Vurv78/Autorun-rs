# Autorun-rs [![Release Shield](https://img.shields.io/github/v/release/Vurv78/Autorun-rs)](https://github.com/Vurv78/Autorun-rs/releases/latest) [![License](https://img.shields.io/github/license/Vurv78/Autorun-rs?color=red)](https://opensource.org/licenses/Apache-2.0) ![CI](https://github.com/Vurv78/Autorun-rs/workflows/Build/badge.svg) [![github/Vurv78](https://img.shields.io/discord/824727565948157963?color=7289DA&label=chat&logo=discord)](https://discord.gg/epJFC6cNsw)

Garrysmod Lua Dumper / Runner, written in Rust.  

## Features
* Dumping all lua scripts to ``C:\Users\<User>\sautorun-rs\lua_dumps\<ServerIP>\..``
* Runtime lua loading through lua_run and lua_openscript in an external console
* Supports x86 and x64 bit
* Scripthook, stop & run scripts before anything runs on you, gives information & functions to assist in a safe separate lua fenv

## Usage
Get an injector (Make sure it's compatible to inject 32/64 bit code depending on your use).  
Use one of the build_win batchfiles to build the output dll, then inject that into garrysmod at the main menu.  

## Notes
* For now this is only supporting Windows.  
  In the future this may be able to run on Linux/OSX but if that does happen I won't be able to actively support it.

## Scripthook
Autorun features scripthook, which means we'll run your script before any other garrysmod script executes to verify if you want the code to run & to let you run your own hook code.

*This runs in a separate fenv from _G, so you can write global variables and it won't affect _G but you can also manually write _G.foo = bar.*

### File Structure

```ruby
C:\Users\<User>\sautorun-rs
├── \autorun.lua # Runs for every script except if hook.lua just ran
├── \hook.lua # Runs *once* before autorun
├── \lua_dumps\ # Each server gets it's own folder named by its IP
│   ├── \192.168.1.1\
│   ├── \192.168.1.2\
│   └── \241241.352.1.3\
└── ...
```

### Fields
Here are the fields for the ``sautorun`` table that gets passed in scripthook. (Todo make this a table)
```ruby
NAME: string # Name of the script. Will be like @lua/this/that.lua
CODE: string # The script itself.
IP: string # Currently connected server
STARTUP: boolean # Whether you're running before autorun or not.
log: function # Prints to the external console.
```

### Examples
autorun.lua
```lua
local script = sautorun.CODE
if script:find("while true do end") then
  sautorun.log("Found a naughty script!")
  return true -- Exit from here & don't run the script
end
```
hook.lua
```lua
sautorun.log( "Connected to server " .. sautorun.IP )
```
