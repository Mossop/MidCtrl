local LrApplication = import "LrApplication"
local LrApplicationView = import "LrApplicationView"
local LrTasks = import "LrTasks"
local LrDevelopController = import "LrDevelopController"
local LrPathUtils = import "LrPathUtils"
local LrFileUtils = import "LrFileUtils"

local Utils = require "Utils"
local logger = require("Logging")("State")
local json = require "json"

local State = {
  running = true,
  params = {},
}

function State:init()
  local paramsFile = LrPathUtils.child(_PLUGIN.path, "params.json")
  local data = LrFileUtils.readFile(paramsFile)

  local success, params = Utils.jsonDecode(logger, data)
  if not success then
    return
  end

  self.params = params
end

function State:getPhotoState()
  local state = {}
  local catalog = LrApplication.activeCatalog()
  local module = LrApplicationView.getCurrentModuleName()
  local photo = catalog:getTargetPhoto()

  if not photo then
    return state
  end

  if module ~= "develop" then
    local photos = catalog:getTargetPhotos()
    if photos[2] then
      return state
    end
  end

  local developState = photo:getDevelopSettings()

  for i, param in ipairs(self.params) do
    local value = nil

    if param["type"] == "develop" then
      value = developState[param["parameter"]]
    end

    if value ~= nil then
      local range = param["max"] - param["min"]
      value = (value - param["min"]) / range
    end

    state[param["parameter"]] = value
  end

  return state
end

function State:getState()
  local state = self:getPhotoState()
  state.module = LrApplicationView.getCurrentModuleName()

  return state
end

function State:disconnect()
  self.running = false
end

function State:watch(listener)
  Utils.runAsync(logger, "watch state", function(context)
    local module = LrApplicationView.getCurrentModuleName()

    while self.running do
      local hasUpdate = false
      local update = {}

      local newModule = LrApplicationView.getCurrentModuleName()
      if newModule ~= module then
        update.module = newModule
        hasUpdate = true
        module = newModule
      end

      if hasUpdate then
        listener(update)
      end

      LrTasks.sleep(0.2)
    end
  end)
end

return State
