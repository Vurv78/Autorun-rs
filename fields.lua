-- Emmylua Autorun definition.
-- Feel free to use in your own plugins.

---@class Autorun
---@field Plugin Plugin
---@field NAME string # Name of script running
---@field STARTUP boolean # True if script is running on autorun.lua
---@field CODE string # Source code of script
---@field CODE_LEN integer # Length of source code
---@field IP string # IP Address of server
---@field PATH string # Path to the currently running script, local to /autorun/. Shouldn't really be used (and definitely not modified.)
Autorun = {}

--- Logs a message to the Autorun console & Logging system (depending on severity)
--- ## Levels
--- * 5 - Trace
--- * 4 - Debug
--- * 3 - Info
--- * 2 - Warning
--- * 1 - Error
---
--- ## Example
--- Logs a warning to the console
--- ```lua
--- Autorun.log("Restricted access to xyz!", 2)
--- ```
---@param message string
---@param severity integer
function Autorun.log(message, severity) end

--- Requires a lua file relative to autorun/scripts OR to the currently running Autorun file.
--- So if you do ``Autorun.require("foo.lua")`` inside of YourPlugin/src/autorun.lua, it will call YourPlugin/src/foo.lua.
--- The require'd file will also contain the ``Autorun`` environment and can return a value to be used by the calling script.
--- Pretty much gmod's include() function.
--- ## Example
--- ```lua
--- local Ret = Autorun.require("bar.lua")
--- ```
---@param path string Path to file to require
---@return ... values Any values returned by the require'd file.
function Autorun.require(path) end

--- Prints any values to the Autorun console, with tables with 3 number values ( {1, 2, 3} ) being treated as colors.
--- All text / values after these colors will be printed in the color specified.
--- Pretty much glua's MsgC but adds a newline as well.
---@vararg string|{[1]: number, [2]: number, [3]: number}|number|userdata|lightuserdata
function Autorun.print(...) end

--- Requires a dynamic link library (.dll) from your autorun/bin folder.
--- Make sure the DLLs are named correctly (e.g. gmcl_name_win<arch>.dll)
--- ```lua
--- local mybin = Autorun.requirebin("CHTTP")
--- ```
---@param path string Path to binary module
---@return ... values Any values returned by the binary module
function Autorun.requirebin(path) end

--- Reads a file relative to current path, OR inside of your plugin's /data/ folder.
--- It can have any file extension, so you could read anything from .txt to .json, .lua, .exe, whatever.
--- ```lua
--- local data = Autorun.readFile("test.txt") -- (Reads autorun/plugins/MyPlugin/data/test.txt OR autorun/plugins/MyPlugin/src/test.txt)
--- ```
---@param path string Path to file
---@return string contents Contents of the file
function Autorun.readFile(path) end

---@class Plugin
---@field Settings table # Key value pairs settings retrieved from plugin.toml
---@field VERSION string # Version of the plugin
---@field AUTHOR string # Author of the plugin
---@field NAME string # Display name of the plugin
---@field DESCRIPTION string # Description of the plugin
---@field DIR string # Plugin's directory name (non-display name)