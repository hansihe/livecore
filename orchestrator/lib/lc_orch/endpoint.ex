defmodule LCOrch.Endpoint do
  use Phoenix.Endpoint, otp_app: :orchestrator

  socket "/socket", LCOrch.Socket, websocket: true

end
