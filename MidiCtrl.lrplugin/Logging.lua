local LrLogger = import "LrLogger"

local Utils = require "Utils"

local lrLogger = LrLogger("MidiCtrl")

if Utils.isDevelopmentBuild() then
  lrLogger:enable({
    -- trace = "logfile",
    debug = "logfile",
    info = "logfile",
    warn = "logfile",
    error = "logfile",
  })
end

local Logger = {}

function Logger:trace(...)
  lrLogger:trace(self.name, unpack(arg))
end

function Logger:debug(...)
  lrLogger:debug(self.name, unpack(arg))
end

function Logger:info(...)
  lrLogger:info(self.name, unpack(arg))
end

function Logger:warn(...)
  lrLogger:warn(self.name, unpack(arg))
end

function Logger:error(...)
  lrLogger:error(self.name, unpack(arg))
end

local function create(name)
  local logger = { name = name }
  setmetatable(logger, { __index = Logger })
  return logger
end

return create
