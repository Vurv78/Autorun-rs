# Autorun-rs
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

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

### TODOs
* Automating JoinServer hook
* Making a lua script in sautorun run before autorun out of the box.

### Notes
This is Windows ONLY. Autorun might've had a chance at being multi-platform but this is absolutely not going to work with OSX/Linux.  
You need to call init_file_steal in your garrysmod main menu in the Autorun terminal, so that files will be dumped.  
Windows 32 bit currently doesn't work for some reason. Might be some x64 only dependencies or something.