local LrPathUtils = import "LrPathUtils"

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

  IPC:init(function (message)
    self:onMessage(message)
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

return Service
