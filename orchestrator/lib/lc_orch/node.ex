defmodule LCOrch.Node do
  use GenServer

  alias LCOrch.Crypto

  def start_link(opts) do
    GenServer.start_link(__MODULE__, opts)
  end

  defstruct [
    state: :handshake1,
    transport: nil,
    client_uuid: nil,
    client_pubkey: nil,
    client_challenge: nil,
  ]

  def init(transport_pid) do
    state = %__MODULE__{
      transport: transport_pid,

      client_uuid: UUID.uuid4(),
    }
    {:ok, state}
  end

  def handle_call(
    {:socket_message, msg = %{"ty" => "client_handshake", "version" => 0}},
    _from,
    state = %{state: :handshake1}
  ) do
    {server_pubkey, server_prikey} = Crypto.gen_ecdsa_keypair()

    %{
      reply: reply,
      client_pubkey: client_pubkey,
      client_challenge: client_challenge,
    } = LCOrch.Node.Handshake.on_client_handshake(
      msg,
      state.client_uuid,
      server_prikey,
      server_pubkey
    )

    state = %{
      state |
      state: :handshake2,
      client_pubkey: client_pubkey,
      client_challenge: client_challenge,
    }

    {:reply, {:reply, reply}, state}
  end

  def handle_call(
    {:socket_message, msg = %{"ty" => "client_handshake_finish"}},
    _from,
    state = %{state: :handshake2}
  ) do

    :ok = LCOrch.Node.Handshake.on_client_handshake_finish(
      msg,
      state.client_pubkey,
      state.client_challenge
    )

    IO.inspect "yay handshake finished!"

    {:reply, :ok, state}
  end

end
