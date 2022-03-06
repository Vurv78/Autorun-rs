-- Replace all 'while true do end' scripts with 'while false do end' 😎
local script = Autorun.CODE
if script:find("while true do end") then
	Autorun.log("Found an evil script!")
	return string.Replace(script, "while true do end", "while false do end")
end