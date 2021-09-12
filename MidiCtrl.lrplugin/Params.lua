local LrApplication = import "LrApplication"
local LrApplicationView = import "LrApplicationView"
local LrDevelopController = import "LrDevelopController"

local Utils = require "Utils"
local logger = require("Logging")("State")

local function currentPhoto(module)
  local catalog = LrApplication.activeCatalog()
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

local function compressRange(value, min, max)
  local range = max - min
  return (value - min) / range
end

local function expandRange(value, min, max)
  local range = max - min
  return (value * range) + min
end

local function setDevelopParam(name, config, value)
  if LrApplicationView.getCurrentModuleName() ~= "develop" then
    LrApplicationView.switchToModule("develop")
  end

  LrDevelopController.startTracking(name)
  LrDevelopController.revealPanel(name)
  LrDevelopController.setValue(name, expandRange(value, config.min, config.max))
end

local function getDevelopParam(name, config, cache)
  if cache.module == "develop" then
    return compressRange(LrDevelopController.getValue(name), config.min, config.max)
  end

  if not cache.developState then
    cache.developState = cache.photo:getDevelopSettings()
  end

  return compressRange(cache.developState[name], config.min, config.max)
end

local function get2012DevelopParam(name, config, cache)
  if cache.module == "develop" then
    return compressRange(LrDevelopController.getValue(name), config.min, config.max)
  end

  if not cache.developState then
    cache.developState = cache.photo:getDevelopSettings()
  end

  return compressRange(cache.developState[name .. "2012"], config.min, config.max)
end

local paramConfigs = {
  Temperature = {
    forPhoto = true,
    min = 2000,
    max = 50000,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  Tint = {
    forPhoto = true,
    min = -150,
    max = 150,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  Exposure = {
    forPhoto = true,
    min = -5,
    max = 5,
    setter = setDevelopParam,
    getter = get2012DevelopParam,
  },
  Highlights = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = get2012DevelopParam,
  },
  Shadows = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = get2012DevelopParam,
  },
  Brightness = {
    forPhoto = true,
    min = -150,
    max = 150,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  Contrast = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = get2012DevelopParam,
  },
  Whites = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = get2012DevelopParam,
  },
  Blacks = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = get2012DevelopParam,
  },
  Texture = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  Clarity = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = get2012DevelopParam,
  },
  Dehaze = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  Vibrance = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  Saturation = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  ParametricDarks = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  ParametricLights = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  ParametricShadows = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  ParametricHighlights = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  ParametricShadowSplit = {
    forPhoto = true,
    min = 10,
    max = 70,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  ParametricMidtoneSplit = {
    forPhoto = true,
    min = 20,
    max = 80,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  ParametricHighlightSplit = {
    forPhoto = true,
    min = 30,
    max = 90,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  SaturationAdjustmentRed = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  SaturationAdjustmentOrange = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  SaturationAdjustmentYellow = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  SaturationAdjustmentGreen = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  SaturationAdjustmentAqua = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  SaturationAdjustmentBlue = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  SaturationAdjustmentPurple = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  SaturationAdjustmentMagenta = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  HueAdjustmentRed = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  HueAdjustmentOrange = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  HueAdjustmentYellow = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  HueAdjustmentGreen = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  HueAdjustmentAqua = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  HueAdjustmentBlue = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  HueAdjustmentPurple = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  HueAdjustmentMagenta = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  LuminanceAdjustmentRed = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  LuminanceAdjustmentOrange = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  LuminanceAdjustmentYellow = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  LuminanceAdjustmentGreen = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  LuminanceAdjustmentAqua = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  LuminanceAdjustmentBlue = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  LuminanceAdjustmentPurple = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  LuminanceAdjustmentMagenta = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  SplitToningShadowHue = {
    forPhoto = true,
    min = 0,
    max = 360,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  SplitToningShadowSaturation = {
    forPhoto = true,
    min = 0,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  ColorGradeShadowLum = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  SplitToningHighlightHue = {
    forPhoto = true,
    min = 0,
    max = 360,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  SplitToningHighlightSaturation = {
    forPhoto = true,
    min = 0,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  ColorGradeHighlightLum = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  ColorGradeMidtoneHue = {
    forPhoto = true,
    min = 0,
    max = 360,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  ColorGradeMidtoneSat = {
    forPhoto = true,
    min = 0,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  ColorGradeMidtoneLum = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  ColorGradeGlobalHue = {
    forPhoto = true,
    min = 0,
    max = 360,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  ColorGradeGlobalSat = {
    forPhoto = true,
    min = 0,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  ColorGradeGlobalLum = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  SplitToningBalance = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  ColorGradeBlending = {
    forPhoto = true,
    min = 0,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  Sharpness = {
    forPhoto = true,
    min = 0,
    max = 150,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  SharpenRadius = {
    forPhoto = true,
    min = 0.5,
    max = 3,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  SharpenDetail = {
    forPhoto = true,
    min = 0,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  SharpenEdgeMasking = {
    forPhoto = true,
    min = 0,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  LuminanceSmoothing = {
    forPhoto = true,
    min = 0,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  LuminanceNoiseReductionDetail = {
    forPhoto = true,
    min = 0,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  LuminanceNoiseReductionContrast = {
    forPhoto = true,
    min = 0,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  ColorNoiseReduction = {
    forPhoto = true,
    min = 0,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  ColorNoiseReductionDetail = {
    forPhoto = true,
    min = 0,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  ColorNoiseReductionSmoothness = {
    forPhoto = true,
    min = 0,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  PostCropVignetteAmount = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  PostCropVignetteMidpoint = {
    forPhoto = true,
    min = 0,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  PostCropVignetteFeather = {
    forPhoto = true,
    min = 0,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  PostCropVignetteRoundness = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  PostCropVignetteStyle = {
    forPhoto = true,
    min = 1,
    max = 3,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  PostCropVignetteHighlightContrast = {
    forPhoto = true,
    min = 0,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  GrainAmount = {
    forPhoto = true,
    min = 0,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  GrainSize = {
    forPhoto = true,
    min = 0,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  GrainFrequency = {
    forPhoto = true,
    min = 0,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  LensProfileDistortionScale = {
    forPhoto = true,
    min = 0,
    max = 200,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  LensProfileVignettingScale = {
    forPhoto = true,
    min = 0,
    max = 200,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  LensManualDistortionAmount = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  DefringePurpleAmount = {
    forPhoto = true,
    min = 0,
    max = 20,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  DefringePurpleHueLo = {
    forPhoto = true,
    min = 0,
    max = 60,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  DefringePurpleHueHi = {
    forPhoto = true,
    min = 40,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  DefringeGreenAmount = {
    forPhoto = true,
    min = 0,
    max = 20,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  DefringeGreenHueLo = {
    forPhoto = true,
    min = 0,
    max = 50,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  DefringeGreenHueHi = {
    forPhoto = true,
    min = 50,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  PerspectiveVertical = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  PerspectiveHorizontal = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  PerspectiveRotate = {
    forPhoto = true,
    min = -10,
    max = 10,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  PerspectiveScale = {
    forPhoto = true,
    min = 50,
    max = 150,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  PerspectiveAspect = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  PerspectiveX = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  PerspectiveY = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  PerspectiveUpright = {
    forPhoto = true,
    min = 0,
    max = 5,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  ShadowTint = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  RedHue = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  RedSaturation = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  GreenHue = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  GreenSaturation = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  BlueHue = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },
  BlueSaturation = {
    forPhoto = true,
    min = -100,
    max = 100,
    setter = setDevelopParam,
    getter = getDevelopParam,
  },

  Module = {
    forPhoto = false,
    setter = function(name, config, value)
      LrApplicationView.switchToModule(value)
    end,
    getter = function(name, config, cache)
      return cache.module
    end,
  },

  Rejected = {
    forPhoto = true,
    needsWriteAccess = true,
    setter = function(name, config, value, photo)
      if value then
        photo:setRawMetadata("pickStatus", -1)
      else
        local status = photo:getRawMetadata("pickStatus")
        if status == -1 then
          photo:setRawMetadata("pickStatus", 0)
        end
      end
    end,
    getter = function(name, config, cache)
      return cache.photo:getRawMetadata("pickStatus") == -1
    end,
  },
  Picked = {
    forPhoto = true,
    needsWriteAccess = true,
    setter = function(name, config, value, photo)
      if value then
        photo:setRawMetadata("pickStatus", 1)
      else
        local status = photo:getRawMetadata("pickStatus")
        if status == 1 then
          photo:setRawMetadata("pickStatus", 0)
        end
      end
    end,
    getter = function(name, config, cache)
      return cache.photo:getRawMetadata("pickStatus") == 1
    end,
  },

  Rating = {
    forPhoto = true,
    needsWriteAccess = true,
    setter = function(name, config, value, photo)
      if value == 0 then
        value = nil
      end
      photo:setRawMetadata("rating", value)
    end,
    getter = function(name, config, cache)
      local rating = cache.photo:getRawMetadata("rating")
      if rating == nil then
        rating = 0
      end
      return rating
    end,
  },
}

local Params = {}

function Params.setParam(name, value)
  local photo = currentPhoto()
  local config = paramConfigs[name]
  if config then
    if config.forPhoto and not photo then
      logger:error("Attempt to set a photo value when no photo is selected", name)
    elseif config.setter then
      if config.needsWriteAccess then
        Utils.runWithWriteAccess(logger, "update metadata", function()
          config.setter(name, config, value, photo)
        end)
      else
        config.setter(name, config, value, photo)
      end
    else
      logger:error("Attempt to set an readonly parameter", name)
    end
  else
    logger:error("Attempt to set an unknown parameter", name)
  end
end

function Params.getParams()
  local params = {}
  local module = LrApplicationView.getCurrentModuleName()
  local cache = {
    photo = currentPhoto(module),
    module = module,
  }

  for name, config in pairs(paramConfigs) do
    if not config.forPhoto or cache.photo then
      params[name] = config.getter(name, config, cache)
    end
  end

  return params
end

return Params
