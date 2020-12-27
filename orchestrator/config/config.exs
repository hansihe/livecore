import Mix.Config

config :orchestrator, LCOrch.Endpoint,
  url: [host: "0.0.0.0"]

config :phoenix, :json_library, Jason

import_config "#{Mix.env()}.exs"
