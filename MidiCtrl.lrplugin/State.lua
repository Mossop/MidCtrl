local LrApplicationView = import "LrApplicationView"
local LrTasks = import "LrTasks"
local LrDevelopController = import "LrDevelopController"

local Utils = require "Utils"
local logger = require("Logging")("State")
local json = require "json"

local DevelopParams = {
  "Temperature",
  "Tint",
  "Exposure",
  "Highlights", 	-- (controls Recovery in Version 1 and Version 2)
  "Shadows", 	-- (controls Fill Light in Version 1 and Version 2)
  "Brightness", -- (no effect unless in Version 1 or Version 2)
  "Contrast",
  "Whites", 	-- (no effect in Version 1 and Version 2)
  "Blacks",
  "Texture",
  "Clarity",
  "Dehaze",
  "Vibrance",
  "Saturation",

 -- tonePanel
  "ParametricDarks",
  "ParametricLights",
  "ParametricShadows",
  "ParametricHighlights",
  "ParametricShadowSplit",
  "ParametricMidtoneSplit",
  "ParametricHighlightSplit",

  -- mixerPanel
  -- HSL / Color
  "SaturationAdjustmentRed",
  "SaturationAdjustmentOrange",
  "SaturationAdjustmentYellow",
  "SaturationAdjustmentGreen",
  "SaturationAdjustmentAqua",
  "SaturationAdjustmentBlue",
  "SaturationAdjustmentPurple",
  "SaturationAdjustmentMagenta",
  "HueAdjustmentRed",
  "HueAdjustmentOrange",
  "HueAdjustmentYellow",
  "HueAdjustmentGreen",
  "HueAdjustmentAqua",
  "HueAdjustmentBlue",
  "HueAdjustmentPurple",
  "HueAdjustmentMagenta",
  "LuminanceAdjustmentRed",
  "LuminanceAdjustmentOrange",
  "LuminanceAdjustmentYellow",
  "LuminanceAdjustmentGreen",
  "LuminanceAdjustmentAqua",
  "LuminanceAdjustmentBlue",
  "LuminanceAdjustmentPurple",
  "LuminanceAdjustmentMagenta",
  -- B & W
  "GrayMixerRed",
  "GrayMixerOrange",
  "GrayMixerYellow",
  "GrayMixerGreen",
  "GrayMixerAqua",
  "GrayMixerBlue",
  "GrayMixerPurple",
  "GrayMixerMagenta",

  -- colorGradingPanel
  "SplitToningShadowHue",
  "SplitToningShadowSaturation",
  "ColorGradeShadowLum",
  "SplitToningHighlightHue",
  "SplitToningHighlightSaturation",
  "ColorGradeHighlightLum",
  "ColorGradeMidtoneHue",
  "ColorGradeMidtoneSat",
  "ColorGradeMidtoneLum",
  "ColorGradeGlobalHue",
  "ColorGradeGlobalSat",
  "ColorGradeGlobalLum",
  "SplitToningBalance",
  "ColorGradeBlending",

  -- detailPanel
  "Sharpness",
  "SharpenRadius",
  "SharpenDetail",
  "SharpenEdgeMasking",
  "LuminanceSmoothing",
  "LuminanceNoiseReductionDetail",
  "LuminanceNoiseReductionContrast",
  "ColorNoiseReduction",
  "ColorNoiseReductionDetail",
  "ColorNoiseReductionSmoothness",

  -- effectsPanel
  -- Post-Crop Vignetting
  "PostCropVignetteAmount",
  "PostCropVignetteMidpoint",
  "PostCropVignetteFeather",
  "PostCropVignetteRoundness",
  "PostCropVignetteStyle",
  "PostCropVignetteHighlightContrast",
  -- Grain
  "GrainAmount",
  "GrainSize",
  "GrainFrequency",

  -- lensCorrectionsPanel
  -- Profile
  "LensProfileDistortionScale",
  "LensProfileVignettingScale",
  "LensManualDistortionAmount",
  -- Color
  "DefringePurpleAmount",
  "DefringePurpleHueLo",
  "DefringePurpleHueHi",
  "DefringeGreenAmount",
  "DefringeGreenHueLo",
  "DefringeGreenHueHi",
  -- Manual Perspective
  "PerspectiveVertical",
  "PerspectiveHorizontal",
  "PerspectiveRotate",
  "PerspectiveScale",
  "PerspectiveAspect",
  "PerspectiveX",
  "PerspectiveY",
  "PerspectiveUpright",

  -- calibratePanel
  "ShadowTint",
  "RedHue",
  "RedSaturation",
  "GreenHue",
  "GreenSaturation",
  "BlueHue",
  "BlueSaturation",

  -- Crop Angle
  "straightenAngle",
}

local State = {
  running = true
}

function State:getState()
  return {
    module = LrApplicationView.getCurrentModuleName(),
  }
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

        if newModule == "develop" then
          local params = {}
          for i, param in ipairs(DevelopParams) do
            local min, max = LrDevelopController.getRange(param)
            params[param] = {
              min = min,
              max = max,
            }
          end

          logger:info(json.encode(params))
        end
      end

      if hasUpdate then
        listener(update)
      end

      LrTasks.sleep(0.2)
    end
  end)
end

return State
