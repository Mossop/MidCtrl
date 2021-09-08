local LrSocket = import "LrSocket"
local LrTasks = import "LrTasks"

local Utils = require "Utils"

local Logger = require("Logging")
local logger = Logger("IPC")

local IPC = {
  running = false,
  sender = nil,
  receiver = nil,
  eventHandler = nil,
}

function IPC:init(listener)
  self.running = true
  self.eventHandler = listener

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
    local disconnected = false

    logger:trace("listening", port)

    LrSocket.bind({
      functionContext = context,
      plugin = _PLUGIN,
      port = port,
      mode = "send",

      onConnected = function(socket)
        logger:debug("connected")
        self.sender = socket

        if self.receiver then
          self.eventHandler("connected")
        end
      end,

      onClosed = function(socket)
        self.sender = nil
        disconnected = true

        logger:debug("closed")

        if self.running then
          self.eventHandler("disconnected")
          self:startSender(port)
        end
      end,

      onError = function(socket, err)
        self.sender = nil

        if err ~= "timeout" then
          disconnected = true

          logger:error("onError", err)

          if self.running then
            self.eventHandler("disconnected")

            LrTasks.sleep(0.5)
            self:startSender(port)
          end

          return
        end

        if self.running then
          socket:reconnect()
        end
      end,
    })

    while self.running and not disconnected do
      LrTasks.sleep(0.2)
    end

    if self.sender and not disconnected then
      self.sender:close()
    end
  end)
end

function IPC:startReceiver(port)
  local logger = Logger("IPC Receiver")
  Utils.runAsync(logger, "receive ipc", function(context)
    logger:trace("listening", port)

    LrSocket.bind({
      functionContext = context,
      plugin = _PLUGIN,
      port = port,
      mode = "receive",

      onConnected = function(socket)
        logger:debug("connected")
        self.receiver = socket

        if self.sender then
          self.eventHandler("connected")
        end
      end,

      onMessage = function(socket, data)
        logger:trace("received message", data)
        local success, message = Utils.jsonDecode(logger, data)
        if not success then
          logger:error("invalid message", message.code, message.name)
          return
        end

        self.eventHandler("message", message)
      end,

      onClosed = function(socket)
        self.receiver = nil
        if self.sender then
          self.sender:close()
        end

        logger:debug("closed")

        if self.running then
          socket:reconnect()
        end
      end,

      onError = function(socket, err)
        self.receiver = nil
        if self.sender then
          self.sender:close()
        end

        if err ~= "timeout" then
          logger:error("onError", err)
          LrTasks.sleep(0.5)
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
