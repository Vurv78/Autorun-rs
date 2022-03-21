-- Emmylua Autorun definition.
-- Feel free to use in your own plugins.

---@class Autorun
---@field Plugin Plugin
---@field NAME string # Name of script running
---@field STARTUP boolean # True if script is running on autorun.lua
---@field CODE string # Source code of script
---@field CODE_LEN integer # Length of source code
---@field IP string # IP Address of server
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

--- Requires a lua file relative to autorun/scripts. Does not work with the plugin system yet.
--- Pretty much gmod's include() function.
--- ## Example
--- ```lua
--- local Ret = Autorun.require("bar.lua")
--- ```
---@param path string
---@return any
function Autorun.require(path) end

--- Prints any values to the Autorun console, with tables with 3 number values ( {1, 2, 3} ) being treated as colors.
--- All text / values after these colors will be printed in the color specified.
--- Pretty much glua's MsgC but adds a newline as well.
---@vararg string|{[1]: number, [2]: number, [3]: number}|number|userdata|lightuserdata
function Autorun.print(...) end

---@class Plugin
---@field Settings table # Key value pairs settings retrieved from plugin.toml
---@field VERSION string # Version of the plugin
---@field AUTHOR string # Author of the plugin
---@field NAME string # Display name of the plugin
---@field DESCRIPTION string # Description of the plugin