local LrApplication = import "LrApplication"
local LrApplicationView = import "LrApplicationView"
local LrTasks = import "LrTasks"
local LrDevelopController = import "LrDevelopController"
local LrPathUtils = import "LrPathUtils"
local LrFileUtils = import "LrFileUtils"

local Utils = require "Utils"
local logger = require("Logging")("State")
local json = require "json"

local function currentPhoto()
  local catalog = LrApplication.activeCatalog()
  local module = LrApplicationView.getCurrentModuleName()
  local photo = catalog:getTargetPhoto()

  if not photo then
    return nil
  end

  if module ~= "develop" then
    local photos = catalog:getTargetPhotos()
    if photos[2] then
      return nil
    end
  end

  return photo
end

local State = {
  running = true,
  params = {},
  state = {},
}

function State:init(listener)
  self.listener = listener

  local paramsFile = LrPathUtils.child(_PLUGIN.path, "params.json")
  local data = LrFileUtils.readFile(paramsFile)

  local success, params = Utils.jsonDecode(logger, data)
  if success then
    self.params = params
  end

  self:watch()
end

function State:buildPhotoState(photo)
  Utils.runAsync(logger, "build photo state", function()
    local state = {}
    local module = LrApplicationView.getCurrentModuleName()

    for i, param in ipairs(self.params) do
      local value = json.null

      if photo and param["type"] == "develop" and module == "develop" then
        value = LrDevelopController.getValue(param["parameter"])
        local range = param["max"] - param["min"]
        value = (value - param["min"]) / range
      end

      state[param["parameter"]] = value
    end

    self:setStates(state)
  end)
end

function State:setStates(newState)
  local changed = false
  local updates = {}

  for k, v in pairs(newState) do
    if self.state[k] ~= v then
      self.state[k] = v
      updates[k] = v
      changed = true
    end
  end

  if changed then
    self.listener(updates)
  end
end

function State:rebuildState()
  self.state = {}

  local photo = currentPhoto()
  if photo then
    self:buildPhotoState(photo)
  end

  self:setStates({
    module = LrApplicationView.getCurrentModuleName()
  })
end

function State:getState()
  return self.state
end

function State:disconnect()
  self.running = false
end

function State:enterModule(context, module)
  self:setStates({
    module = LrApplicationView.getCurrentModuleName()
  })

  if module == "develop" then
    LrDevelopController.addAdjustmentChangeObserver(context, {}, function()
      self:buildPhotoState(currentPhoto())
    end)
  end
end

local function photosMatch(a, b)
  if a == nil then
    if b == nil then
      return true
    else
      return false
    end
  elseif b == nil then
    return false
  else
    return a.localIdentifier == b.localIdentifier
  end
end

function State:watch()
  Utils.runAsync(logger, "watch state", function(context)
    local module = LrApplicationView.getCurrentModuleName()
    local photo = currentPhoto()

    self:enterModule(context, module)

    while self.running do
      local newModule = LrApplicationView.getCurrentModuleName()
      local newPhoto = currentPhoto()

      if newModule ~= module then
        module = newModule
        self:enterModule(context, module)
        photo = newPhoto
        self:buildPhotoState(photo)
      elseif not photosMatch(newPhoto, photo) then
        photo = newPhoto
        self:buildPhotoState(photo)
      end

      LrTasks.sleep(0.2)
    end
  end)
end

return State
