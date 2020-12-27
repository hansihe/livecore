use Mix.Config

config :orchestrator, LCOrch.Endpoint,
  http: [port: 4000],
  debug_errors: true,
  check_origin: false
