local LrApplicationView = import "LrApplicationView"
local LrTasks = import "LrTasks"
local LrDevelopController = import "LrDevelopController"

local Utils = require "Utils"
local logger = require("Logging")("State")
local json = require "json"
local Params = require "Params"

local State = {
  running = false,
  state = {},
}

function State:init(listener)
  self.listener = listener
end

function State:rebuildState()
  self.state = {}
  self:updateState()
end

function State:updateState()
  Utils.runAsync(logger, "updateState", function(context)
    local state = Params.getParams()
    local hasUpdates = false
    local values = {}

    local unusedState = {}
    for param, value in pairs(state) do
      if value ~= self.state[param] then
        table.insert(values, { parameter = param, value = value })
        hasUpdates = true
        self.state[param] = value
      else
        unusedState[param] = value
      end
    end

    for param, value in pairs(unusedState) do
      if state[param] == nil then
        table.insert(values, { parameter = param, value = json.null })
        hasUpdates = true
        self.state[param] = nil
      end
    end

    if hasUpdates then
      self.listener(values)
    end
  end)
end

function State:setValue(name, value)
  Params.setParam(name, value)
end

function State:getState()
  return self.state
end

function State:enterModule(context, module)
  if module == "develop" then
    LrDevelopController.addAdjustmentChangeObserver(context, {}, function()
      self:updateState()
    end)
  end
end

function State:connected()
  if self.running then
    return
  end

  self.running = true
  logger:debug("Starting state updates")

  Utils.runAsync(logger, "watch state", function(context)
    local module = LrApplicationView.getCurrentModuleName()
    self:enterModule(context, module)

    while self.running do
      local newModule = LrApplicationView.getCurrentModuleName()

      if newModule ~= module then
        module = newModule
        self:enterModule(context, module)
      end

      self:updateState()
      LrTasks.sleep(0.2)
    end

    logger:debug("Stopped state updates")
  end)
end

function State:disconnected()
  self.running = false
end

return State
