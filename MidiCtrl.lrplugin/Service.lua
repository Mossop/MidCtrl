local LrDialogs = import "LrDialogs"
local LrShell = import "LrShell"
local LrSelection = import "LrSelection"

local State = require "State"
local Utils = require "Utils"
local logger = require("Logging")("Service")

local IPC = require "IPC"

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
    elseif event == "disconnected" then
      self.onDisconnected()
    end
  end)

  Service:launchBinary()
end

function Service:launchBinary()
  if not Utils.isDevelopmentBuild() then
    LrShell.openPathsViaCommandLine({}, Utils.binary, "embedded")
  end
end

function Service:shutdown()
  if not self.running then
    return
  end
  logger:trace("Shutdown")

  self.running = false
  IPC:disconnect()
  State:disconnected()
end

function Service:performAction(action)
  local actions = {
    nextPhoto = function()
      LrSelection.nextPhoto()
    end,

    previousPhoto = function()
      LrSelection.previousPhoto()
    end,
  }

  local cb = actions[action.type]
  if cb then
    cb()
  end
end

function Service:onMessage(message)
  Utils.runAsync(logger, "message handling", function()
    local callbacks = {
      notification = function()
        LrDialogs.showBezel(message.message)
      end,

      setValue = function()
        State:setValue(message.name, message.value)
      end,

      action = function()
        self:performAction(message)
      end,
    }

    local cb = callbacks[message.type]
    if cb then
      cb()
    end
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
    State:connected()
  end)
end

function Service:onDisconnected()
  State:disconnected()
end

return Service
