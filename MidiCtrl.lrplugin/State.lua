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

function State:buildLibraryState()
end

function State:buildPhotoState(photo)
  if photo then
    local pickStatus = photo:getRawMetadata("pickStatus")
    local rating = photo:getRawMetadata("rating")
    if rating == nil then
      rating = 0
    end

    self:setStates({
      isPicked = pickStatus == 1,
      isRejected = pickStatus == -1,
      isVirtualCopy = photo:getRawMetadata("isVirtualCopy"),
      isInStack = photo:getRawMetadata("isInStackInFolder"),
      isVideo = photo:getRawMetadata("isVideo"),
      rating = rating,
    })
  else
    self:setStates({
      isPicked = json.null,
      isRejected = json.null,
      isVirtualCopy = json.null,
      isInStack = json.null,
      isVideo = json.null,
      rating = json.null,
    })
  end
end

function State:buildDevelopState(photo)
  Utils.runAsync(logger, "build photo state", function()
    local state = {}
    local module = LrApplicationView.getCurrentModuleName()
    local developState = photo:getDevelopSettings()

    for i, param in ipairs(self.params) do
      local value = json.null

      if photo and param["type"] == "develop" then
        if module == "develop" then
          value = LrDevelopController.getValue(param["parameter"])
        elseif developState[param["parameter"] .. "2012"] ~= nil then
          value = developState[param["parameter"] .. "2012"]
        elseif developState[param["parameter"]] ~= nil then
          value = developState[param["parameter"]]
        else
          logger:warn("Couldn't find develop parameter", param["parameter"])
        end

        if value ~= json.null then
          local range = param["max"] - param["min"]
          value = (value - param["min"]) / range
        end
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
    self:buildDevelopState(photo)
    self:buildPhotoState(photo)
    self:buildLibraryState()
  end

  self:setStates({
    module = LrApplicationView.getCurrentModuleName()
  })
end

function State:setValue(name, value)
  local module = LrApplicationView.getCurrentModuleName()
  local photo = currentPhoto()
  if not photo then
    return
  end

  for i, param in ipairs(self.params) do
    if param["parameter"] == name then
      if param["type"] == "develop" then
        local range = param["max"] - param["min"]
        local value = (value * range) + param["min"]

        if module ~= "develop" then
          LrApplicationView.switchToModule("develop")
        end

        LrDevelopController.startTracking(name)
        LrDevelopController.setValue(name, value)
      end

      return
    end
  end

  if name == "module" then
    LrApplicationView.switchToModule(value)
    return
  end

  Utils.runWithWriteAccess(logger, "update metadata", function()
    if name == "isRejected" then
      if value then
        photo:setRawMetadata("pickStatus", -1)
      else
        local status = photo:getRawMetadata("pickStatus")
        if status == -1 then
          photo:setRawMetadata("pickStatus", 0)
        end
      end
    end

    if name == "isPicked" then
      if value then
        photo:setRawMetadata("pickStatus", 1)
      else
        local status = photo:getRawMetadata("pickStatus")
        if status == 1 then
          photo:setRawMetadata("pickStatus", 0)
        end
      end
    end

    if name == "rating" then
        if value == 0 then
          value = nil
        end
        photo:setRawMetadata("rating", value)
    end
  end)
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
      self:buildDevelopState(currentPhoto())
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
        self:buildDevelopState(photo)
        self:buildLibraryState()
      elseif not photosMatch(newPhoto, photo) then
        if newPhoto then
          logger:debug("Photo changed to", newPhoto:getFormattedMetadata("fileName"))
        else
          logger:debug("No photo selection")
        end
        photo = newPhoto
        self:buildDevelopState(photo)
        self:buildLibraryState()
      end

      self:buildPhotoState(photo)

      LrTasks.sleep(0.2)
    end
  end)
end

return State
