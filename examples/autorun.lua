local ERROR, WARN, INFO, DEBUG, TRACE = 1, 2, 3, 4, 5
Autorun.log( "Connected to server " .. Autorun.IP, DEBUG )

-- Change your country flag to North Korea and Operating System to ``Other``
-- Note this is easy to detect since anticheats will see that the system functions are lua functions, not C ones.
-- (You'd need to detour a lot more to make this undetected.)
jit.os = "Other"
function system.IsLinux() return true end
function system.IsOSX() return false end
function system.IsWindows() return false end
function system.GetCountry() return "KP" end