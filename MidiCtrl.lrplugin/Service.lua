local LrPathUtils = import "LrPathUtils"
local LrDialogs = import "LrDialogs"

local State = require "State"
local Utils = require "Utils"
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

  State:init(function(update)
    IPC:send({
      type = "state",
      state = update,
    })
  end)

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
  State:disconnect()
end

function Service:onMessage(message)
  Utils.runAsync(logger, "connected", function()
  end)
end

function Service:onConnected()
  Utils.runAsync(logger, "connected", function()
    logger:trace("Connected")
    LrDialogs.showBezel("Connected to MidiCtrl")

    IPC:send({
      type = "reset",
    })

    State:rebuildState()
  end)
end

return Service
