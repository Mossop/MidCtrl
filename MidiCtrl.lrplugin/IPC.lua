local Utils = require "Utils"
local Socket = require "Socket"

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

  self:startReceiver(61328)
  self:startSender(61327)
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

  self.sender:send(data)
  return true
end

function IPC:startSender(port)
  local logger = Logger("IPC Sender")

  Socket.connectSender(logger, port, {
    onConnecting = function(socket)
      self.sender = socket
    end,

    onConnected = function()
      if self.receiver and self.receiver.connected then
        self.eventHandler("connected")
      end
    end,

    onDisconnected = function(socket, willReconnect)
      if self.receiver and self.receiver.connected then
        self.eventHandler("disconnected")
      end

      if self.receiver then
        if willReconnect then
          self.receiver:reconnect()
        else
          self.receiver:close()
        end
      end
    end,
  })
end

function IPC:startReceiver(port)
  local logger = Logger("IPC Receiver")

  Socket.connectReceiver(logger, port, {
    onConnecting = function(socket)
      self.receiver = socket
    end,

    onConnected = function()
      if self.sender and self.sender.connected then
        self.eventHandler("connected")
      end
    end,

    onMessage = function(socket, data)
      local success, message = Utils.jsonDecode(logger, data)
      if not success then
        logger:error("invalid message", message.code, message.name)
        return
      end

      self.eventHandler("message", message)
    end,

    onDisconnected = function(socket, willReconnect)
      if self.sender and self.sender.connected then
        self.eventHandler("disconnected")
      end

      if self.sender then
        if willReconnect then
          self.sender:reconnect()
        else
          self.sender:close()
        end
      end
    end,
  })
end

function IPC:shutdown()
  if not self.running then
    return
  end

  self.running = false

  local sender, receiver = self.sender, self.receiver
  self.sender = nil
  self.receiver = nil

  if sender then
    sender:close()
  end

  if receiver then
    receiver:close()
  end
end

return IPC
