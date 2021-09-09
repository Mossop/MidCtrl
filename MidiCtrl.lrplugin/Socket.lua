local LrSocket = import "LrSocket"
local LrTasks = import "LrTasks"

local Utils = require "Utils"

local function call(logger, cb, ...)
  if not cb then
    return
  end

  Utils.safeCall(logger, "callback", function()
    cb(unpack(arg))
  end)
end

local id = 0
local function uuid()
  id = id + 1
  return id
end

local function connectSocket(socket)
  Utils.runAsync(socket.logger, "socket connect", function(context)
    local socketId = uuid()
    socket.socketId = socketId
    socket.lrSocket = LrSocket.bind({
      functionContext = context,
      plugin = _PLUGIN,
      port = socket.port,
      mode = socket.mode,

      onConnecting = function(lrSocket, port)
        if socket.socketId ~= socketId then
          return
        end

        socket:onConnecting(port)
      end,

      onConnected = function(lrSocket, port)
        if socket.socketId ~= socketId then
          return
        end

        socket:onConnected(port)
      end,

      onMessage = function(lrSocket, message)
        if socket.socketId ~= socketId then
          return
        end

        socket:onMessage(message)
      end,

      onClosed = function(lrSocket)
        if socket.socketId ~= socketId then
          return
        end

        socket:onClosed()
      end,

      onError = function(lrSocket, err)
        if socket.socketId ~= socketId then
          return
        end

        if err == "timeout" then
          socket:onTimeout()
        else
          socket:onError(err)
        end
      end,
    })

    while not socket.closed and socket.socketId == socketId do
      LrTasks.sleep(0.2)
    end

    if socket.socketId == socketId then
      socket.logger:trace("Socket loop terminated")
      socket.callbacks = nil
      socket.lrSocket = nil
      socket.connected = false
    end
  end)
end

local Socket = {
}

function Socket:onConnecting(port)
  self.reconnecting = false

  if self.seenConnecting then
    return
  end

  self.seenConnecting = true
  self.logger:trace("listening", port)

  call(self.logger, self.callbacks.onConnecting, self, port)
end

function Socket:onConnected(port)
  self.connected = true
  self.logger:debug("connected", port)

  call(self.logger, self.callbacks.onConnected, self)
end

function Socket:onMessage(message)
  self.logger:trace("received message", message)

  call(self.logger, self.callbacks.onMessage, self, message)
end

function Socket:onClosed()
  local wasConnected = self.connected
  local willReconnect = not self.closed and (self.reconnecting or wasConnected)

  if wasConnected then
    self.logger:debug("closed", willReconnect)
    self.connected = false

    if not self.reconnecting then
      call(self.logger, self.callbacks.onDisconnected, self, willReconnect)
    end
  end

  if willReconnect then
    connectSocket(self)
  else
    self.logger:debug("Closing down socket")
    self.closed = true
    self.socketId = nil
    self.lrSocket = nil
  end
end

function Socket:onTimeout()
  local wasConnected = self.connected
  self.connected = false

  if wasConnected then
    call(self.logger, self.callbacks.onDisconnected, self, not self.closed)
  end

  if not self.closed then
    self.lrSocket:reconnect()
  end
end

function Socket:onError(err)
  local wasConnected = self.connected
  self.connected = false
  self.logger:debug("error", err)

  if wasConnected then
    call(self.logger, self.callbacks.onDisconnected, self, not self.closed)
  else
    LrTasks.sleep(0.5)
  end

  if not self.closed then
    self.lrSocket:reconnect()
  end
end

function Socket:send(message)
  if not self.connected then
    self.logger:trace("Ignoring attempt to send while not connected")
    return
  end

  self.lrSocket:send(message .. "\n")
end

function Socket:reconnect()
  if self.closed then
    return
  end

  self.reconnecting = true
  Utils.runAsync(self.logger, "socket reconnect", function()
    self.lrSocket:close()
  end)
end

function Socket:close()
  if self.closed then
    return
  end

  self.closed = true
  self.socketId = nil
  local lrSocket = self.lrSocket
  self.lrSocket = nil
  self.connected = false

  if lrSocket then
    Utils.runAsync(self.logger, "socket close", function()
      lrSocket:close()
    end)
  end
end

local function initSocket(logger, mode, port, callbacks)
  local socket = {
    logger = logger,
    port = port,
    mode = mode,
    callbacks = callbacks,
    socketId = nil,
    lrSocket = nil,
    connected = false,
    closed = false,
    seenConnecting = false,
    reconnecting = false,
  }
  setmetatable(socket, { __index = Socket })

  connectSocket(socket)
end

local function createSender(logger, port, callbacks)
  initSocket(logger, "send", port, callbacks)
end

local function createReceiver(logger, port, callbacks)
  initSocket(logger, "receive", port, callbacks)
end

return {
  connectSender = createSender,
  connectReceiver = createReceiver,
}
