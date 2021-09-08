return {
  LrSdkVersion = 10.0,
  LrSdkMinimumVersion = 10.0,

  LrToolkitIdentifier = "com.oxymoronical.midictrl",

  LrPluginName = LOC "$$$/LrMidiCtrl/PluginName=LrMidiCtrl",
  LrPluginInfoUrl = "https://github.com/Mossop/MidiCtrl/",

  LrInitPlugin = "Init.lua",
  LrDisablePlugin = "Shutdown.lua",
  LrShutdownPlugin = "Shutdown.lua",
  LrForceInitPlugin = true,

  LrExportMenuItems = {
    {
      title = "Test",
      file = "Init.lua",
    },
  },

  VERSION = { major = 9, minor = 9, revision = 9, build = 9, },
}
