local LrSocket = import "LrSocket"
local LrTasks = import "LrTasks"

local Utils = require "Utils"

local Logger = require("Logging")
local logger = Logger("IPC")

local IPC = {
  running = false,
  sender = nil,
  receiver = nil,
  onMessage = nil,
}

function IPC:init(listener)
  self.running = true
  self.onMessage = listener

  IPC:startReceiver(61328)
  IPC:startSender(61327)
end

function IPC:send(message)
  if not self.sender then
    return false, {
      code = "notConnected",
      name = "Not connected",
    }
  end

  local success, data = Utils.jsonEncode(logger, message)
  if not success then
    return success, data
  end

  self.sender:send(data .. "\n")
  return true
end

function IPC:startSender(port)
  local logger = Logger("IPC Sender")
  Utils.runAsync(logger, "send ipc", function(context)
    LrSocket.bind({
      functionContext = context,
      plugin = _PLUGIN,
      port = port,
      mode = "send",

      onConnecting = function(socket, port)
        logger:trace("listening", port)
      end,

      onConnected = function(socket)
        logger:debug("connected")
        self.sender = socket
      end,

      onClosed = function(socket)
        self.sender = nil
        logger:debug("closed")

        if self.running then
          socket:reconnect()
        end
      end,

      onError = function(socket, err)
        self.sender = nil

        if err ~= "timeout" then
          logger:error("onError", err)
        end

        if self.running then
          socket:reconnect()
        end
      end,
    })

    while self.running do
      LrTasks.sleep(0.2)
    end

    if self.sender then
      self.sender:close()
    end
  end)
end

function IPC:startReceiver(port)
  local logger = Logger("IPC Receiver")
  Utils.runAsync(logger, "receive ipc", function(context)
    LrSocket.bind({
      functionContext = context,
      plugin = _PLUGIN,
      port = port,
      mode = "receive",

      onConnecting = function(socket, port)
        logger:trace("listening", port)
      end,

      onConnected = function(socket)
        logger:debug("connected")
        self.receiver = socket
      end,

      onMessage = function(socket, data)
        logger:trace("received message", data)
        local success, message = Utils.jsonDecode(logger, data)
        if not success then
          logger:error("invalid message", message.code, message.name)
          return
        end

        self.onMessage(message)
      end,

      onClosed = function(socket)
        self.receiver = nil
        logger:debug("closed")

        if self.running then
          socket:reconnect()
        end
      end,

      onError = function(socket, err)
        self.receiver = nil

        if err ~= "timeout" then
          logger:error("onError", err)
        end

        if self.running then
          socket:reconnect()
        end
      end,
    })

    while self.running do
      LrTasks.sleep(0.2)
    end

    if self.receiver then
      self.receiver:close()
    end
  end)
end

function IPC:disconnect()
  if not self.running then
    return
  end

  self.running = false

  if self.sender then
    self.sender:close()
    self.sender = nil
  end

  if self.receiver then
    self.receiver:close()
    self.receiver = nil
  end
end

return IPC
