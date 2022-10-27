[![Autorun](assets/logo.png)](https://github.com/Vurv78/Autorun-rs)
# [![Release Shield](https://img.shields.io/github/v/release/Vurv78/Autorun-rs)](https://github.com/Vurv78/Autorun-rs/releases/latest) [![License](https://img.shields.io/github/license/Vurv78/Autorun-rs?color=red)](https://opensource.org/licenses/Apache-2.0) [![CI](https://github.com/Vurv78/Autorun-rs/workflows/Download/badge.svg)](https://github.com/Vurv78/Autorun-rs/actions/workflows/downloads.yml) [![github/Vurv78](https://img.shields.io/discord/824727565948157963?label=Discord&logo=discord&logoColor=ffffff&labelColor=7289DA&color=2c2f33)](https://discord.gg/yXKMt2XUXm)

> Garrysmod Lua Dumper & Runner, written in Rust.  

## Features
* Dumping all lua scripts to ``C:\Users\<User>\autorun\lua_dumps\<ServerIP>\..`` (asynchronously to avoid i/o lag)
* Runtime lua loading through ``lua_run`` and ``lua_openscript`` in an external console
* Supports both 32* and 64 bit branches (*See [#22](https://github.com/Vurv78/Autorun-rs/issues/22))
* Running a script before autorun (``autorun.lua``), to detour and bypass any 'anticheats'
* Scripthook, stop & run scripts before anything runs on you, gives information & functions to assist in a safe separate lua environment
* File logging (to ``autorun/logs``)
* Plugin system (``autorun/plugins``)
* [Settings using TOML](autorun/src/configs/settings.toml)

## 🤔 Usage
### 🧩 Menu Plugin
Autorun can also be used as a menu plugin / required from lua automatically from the menu state.
1. Put [the dll](#%EF%B8%8F-downloading) ``gmsv_autorun_win<arch>.dll`` file into your ``garrysmod/lua/bin`` folder.
2. Add ``require("autorun")`` at the bottom of ``garrysmod/lua/menu/menu.lua``  
**It will now run automatically when garrysmod loads at the menu.**

### 💉 Injecting
The traditional (but more inconvenient) method to use this is to just inject it.
1. Get an injector (Make sure it's compatible to inject 32/64 bit code depending on your use).  
2. Inject [the dll](#%EF%B8%8F-downloading) into gmod while you're in the menu

## 📜 Scripthook
Autorun features scripthook, which means we'll run your script before any other garrysmod script executes to verify if you want the code to run by running your own hook script.
*This runs in a separate environment from ``_G``, so to modify globals, do ``_G.foo = bar``

Also note that if you are running in ``autorun.lua`` Functions like ``http.Fetch`` & ``file.Write`` won't exist.  
Use their C counterparts (``HTTP`` and ``file.Open``)

__See an example project using the scripthook [here](https://github.com/Vurv78/Safety).__

### 📁 File Structure
```golo
C:\Users\<User>\autorun
├── \autorun.lua # Runs *once* before autorun
├── \hook.lua # Runs for every script
├── \lua_dumps\ # Each server gets a folder with their IP as the name.
│   ├── \192.168.1.55_27015\
│   └── \X.Y.Z.W_PORT\
├── \logs\ # Logs are saved here
│   └── YYYY-MM-DD.log
├── \bin\ # Store binary modules to be used with Autorun.requirebin
│   └── gmcl_vistrace_win64.dll
├── \plugins\ # Folder for Autorun plugins, same behavior as above autorun and hook.lua, but meant for plugin developers.
│   └── \Safety\
│       ├── \src\
|       |   ├── autorun.lua
|       |   └── hook.lua
│       └── plugin.toml
├── settings.toml # See autorun/src/configs/settings.toml
└── ...
```

### 🗃️ Fields
You can find what is passed to the scripthook environment in [fields.lua](fields.lua) as an EmmyLua definitions file.  
This could be used with something like a vscode lua language server extension for intellisense 👍

### ✍️ Examples
__hook.lua__  
This file runs before every single lua script run on your client from addons and servers.
You can ``return true`` to not run the script, or a string to replace it.
```lua
-- Replace all 'while true do end' scripts with 'while false do end' 😎
local script = Autorun.CODE
if script:find("while true do end") then
	Autorun.log("Found an evil script!")
	return string.Replace(script, "while true do end", "while false do end")
end
```

You can find more [here](examples)

## ⬇️ Downloading
### 🦺 Stable
You can get a 'stable' release from [the releases](https://github.com/Vurv78/Autorun-rs/releases/latest).
### 🩸 Bleeding Edge
You can get the absolute latest download (from code in the repo) in [the Github Actions tab](https://github.com/Vurv78/Autorun-rs/actions/workflows/downloads.yml)  
Note it may not work as expected (but I'd advise to try this out before trying to report an issue to see if it has been fixed)

__If you are using this as a menu plugin 🧩, make sure the DLL is named ``gmsv_autorun_win<arch>.dll``__

## 🛠️ Building
You may want to build this yourself if you want to make changes / contribute (or don't trust github actions for whatever reason..)
1. [Setup Rust & Cargo](https://www.rust-lang.org/learn/get-started)
2. Use ``build_win_32.bat`` or ``build_win_64.bat``.  
