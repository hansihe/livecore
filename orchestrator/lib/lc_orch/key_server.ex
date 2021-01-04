defmodule LCOrch.KeyServer do
  use GenServer
  require Logger

  alias LCOrch.Crypto

  def start_link(_arg) do
    GenServer.start_link(__MODULE__, nil, name: __MODULE__)
  end

  def get_key() do
    GenServer.call(__MODULE__, :get_key)
  end

  def init(_arg) do
    {pubkey, prikey} = Crypto.gen_ecdsa_keypair()
    Logger.info("generated orchestrator keypair " <> :base64.encode(pubkey))

    state = %{
      pubkey: pubkey,
      prikey: prikey,
    }

    {:ok, state}
  end

  def handle_call(:get_key, _from, state) do
    {:reply, state, state}
  end

end
