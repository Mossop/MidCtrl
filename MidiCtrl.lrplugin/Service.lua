local LrPathUtils = import "LrPathUtils"
local LrDialogs = import "LrDialogs"
local LrApplicationView = import "LrApplicationView"

local logger = require("Logging")("Service")

local IPC = require "IPC"

local function join(base, ...)
  local result = base
  for i, v in ipairs(arg) do
    result = LrPathUtils.child(result, v)
  end
  return result
end

local Service = {
  running = false
}

function Service:init()
  logger:trace("Startup")
  self.running = true

  IPC:init(function (event, message)
    if event == "message" then
      self:onMessage(message)
    elseif event == "connected" then
      self:onConnected()
    end
  end)

  Service:launchBinary()
end

function Service:launchBinary()
  local root = LrPathUtils.parent(_PLUGIN.path)
  local binary = join(root, "target", "debug", "midi-ctrl")

  -- LrShell.openFilesInApp({}, binary)
end

function Service:shutdown()
  if not self.running then
    return
  end
  logger:trace("Shutdown")

  self.running = false
  IPC:disconnect()
end

function Service:onMessage(message)
end

function Service:onConnected()
  LrDialogs.showBezel("Connected to MidiCtrl")

  IPC:send({
    type = "state",
    state = {
      module = LrApplicationView.getCurrentModuleName(),
    },
  })
end

return Service
