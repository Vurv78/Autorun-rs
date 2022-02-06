# ``Autorun-rs`` [![Release Shield](https://img.shields.io/github/v/release/Vurv78/Autorun-rs)](https://github.com/Vurv78/Autorun-rs/releases/latest) [![License](https://img.shields.io/github/license/Vurv78/Autorun-rs?color=red)](https://opensource.org/licenses/Apache-2.0) ![CI](https://github.com/Vurv78/Autorun-rs/workflows/Build/badge.svg) [![github/Vurv78](https://img.shields.io/discord/824727565948157963?label=Discord&logo=discord&logoColor=ffffff&labelColor=7289DA&color=2c2f33)](https://discord.gg/yXKMt2XUXm)

Garrysmod Lua Dumper & Runner, written in Rust.  

## Features
* Dumping all lua scripts to ``C:\Users\<User>\autorun\lua_dumps\<ServerIP>\..``
* Runtime lua loading through ``lua_run`` and ``lua_openscript`` in an external console
* Supports both 32 and 64 bit branches
* Running a script before autorun (``autorun.lua``), to detour and bypass any 'anticheats'
* Scripthook, stop & run scripts before anything runs on you, gives information & functions to assist in a safe separate lua environment

## Usage
### Injecting
The traditional (but more inconvenient) method to use this is to just inject it.
1. Get an injector (Make sure it's compatible to inject 32/64 bit code depending on your use).  
2. Get the Autorun-rs DLL, either by building it yourself or by getting one from the [releases](https://github.com/Vurv78/Autorun-rs/releases)
3. Inject the DLL into GMod in the Menu
### Menu Plugin
Autorun can also be used as a menu plugin / required from lua. Just as any other scripthook, it is ran from the menu state.  
1. Put the ``gmsv_autorun_win<arch>.dll`` file into your ``garrysmod/lua/bin`` folder.
2. Add ``require("autorun")`` at the bottom of ``garrysmod/lua/menu/menu.lua``  
**It will now run automatically when garrysmod loads at the menu.**

## Scripthook
Autorun features scripthook, which means we'll run your script before any other garrysmod script executes to verify if you want the code to run by running your own hook script.
*This runs in a separate environment from _G, so to modify globals, do _G.foo = bar*

Also note that if you are running in ``autorun.lua`` You will not have access to functions created by glua, like ``http.Fetch`` & ``file.Write``.  
Use the C equivalents (``HTTP`` and ``file.Open``)

See an example project using the scripthook [here](https://github.com/Vurv78/Safety).

### File Structure

```golo
C:\Users\<User>\autorun
├── \autorun.lua # Runs *once* before autorun
├── \hook.lua # Runs for every script (including init.lua, which triggers autorun.lua)
├── \lua_dumps\ # Each server gets it's own folder named by its IP
│   ├── \192.168.1.1\
│   ├── \192.168.1.2\
│   └── \241241.352.1.3\
│   \logs\
│   └── August 02, 2021 01-00 pm.log
└── ...
```

### Fields
Here are the fields for the ``sautorun`` table that gets passed in scripthook.
| Field    | Type             | Description                                                             |
| ---      | ---              | ---                                                                     |
| NAME     | string           | Name of the script, ex: @lua/this/that.lua                              |
| CODE     | string           | The contents of the script                                              |
| IP       | string           | IP of the server you are currently connected to                         |
| STARTUP  | boolean          | Whether the script is running from ``autorun.lua`` (true) or false      |
| log      | function<string, uint?> | A function that logs to your autorun console. Second param is level ascending with urgency, 1 being error, 2 warning, 3, info, 4 debug, 5 trace. Default 3        |
| require | function<string> | Works like gmod's include function. Does not cache like regular lua's require for now. Runs a script local to autorun/scripts and passes the returned values |

### Examples
__hook.lua__  
This file runs before every single lua script run on your client from addons and servers.
```lua
local script = sautorun.CODE
if script:find("while true do end") then
	sautorun.log("Found an evil script!")
	-- Run our modified script that will replace all ``while true do end`` with ``while false do end``. 😎

	return string.Replace(script, "while true do end", "while false do end")

	-- OR: return true to not run the script at all.
end
```
__autorun.lua__  
This will be the first lua script to run on your client when you join a server, use this to make detours and whatnot.
```lua
local ERROR, WARN, INFO, DEBUG, TRACE = 1, 2, 3, 4, 5
sautorun.log( "Connected to server " .. sautorun.IP, DEBUG )
```

## Logging
Autorun features logging under the ``logging`` feature. It is enabled by default.
> Autorun automatically writes logs to a log file whenever you boot up a game for your security and for easy debugging.
> Check the autorun/logs directory for crash dumps & logs if you use something like [Safety](https://github.com/Vurv78/Safety) to log HTTP requests, etc.

## Building
1. [Setup Rust & Cargo](https://www.rust-lang.org/learn/get-started)
2. Use ``build_win_32.bat`` or ``build_win_64.bat``.  
