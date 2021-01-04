defmodule LCOrch.TestUtils.NodeSpawn do
  require Logger

  defstruct [
    port: nil,
    socket: nil,
    uuid: nil,
    pubkey: nil,
    peer_ipc_path: nil,
  ]

  def build() do
    args = [
      "build",
      "--features", "cli,env_logger,tokio/rt-multi-thread,ipc_peer",
      "--bin", "lc_fabric_cli",
    ]
    out = System.cmd("cargo", args, cd: "../fabric_client", stderr_to_stdout: true)
    case out do
      {_output, 0} ->
        Logger.info("Built fabric_client cli successfully")
        :ok
      {output, status} ->
        IO.puts(output)
        Logger.error("Cargo build of fabric_client cli exited with status #{status}")
        :error
    end
  end

  def spawn_process(ipc_socket_path, peer_ipc_path) do
    path = Path.expand("../target/debug/lc_fabric_cli")
    args = [
      ipc_socket_path,
      "--class", "fanout",
      "--fabric-protocol", "ipc",
      "--ipc-peer-bind", peer_ipc_path
    ]
    env = [
      {'RUST_LOG', 'debug'},
    ]

    owner = self()
    ref = make_ref()

    spawn fn ->
      port = Port.open({:spawn_executable, path}, [:binary, :stderr_to_stdout, args: args, env: env])
      send owner, {ref, port}
      print_port_stdout(port, "")
    end

    receive do
      {^ref, port} -> port
    end
  end

  def print_port_stdout(port, acc) do
    receive do
      {^port, {:data, data}} ->
        acc = acc <> data
        parts = String.split(acc, ~r"[\r\n]")
        acc = print_parts(parts)
        print_port_stdout(port, acc)
      {^port, :closed} ->
        nil
    end
  end

  def print_parts([last]), do: last
  def print_parts([head | tail]) do
    Logger.info(String.trim_trailing(head))
    print_parts(tail)
  end

  def spawn() do
    socket_path = "/tmp/" <> UUID.uuid4()
    peer_ipc_path = "/tmp/" <> UUID.uuid4()

    args = [
      :binary,
      # 4 byte length prefixed packets
      {:packet, 4},
      {:ifaddr, {:local, socket_path}},
      {:active, false},
    ]
    {:ok, listen_socket} = :gen_tcp.listen(0, args)

    port = spawn_process(socket_path, peer_ipc_path)

    {:ok, socket} = :gen_tcp.accept(listen_socket)

    :ok = :gen_tcp.close(listen_socket)
    File.rm!(socket_path)

    spawn_link fn ->
      :erlang.process_flag(:trap_exit, true)
      receive do
        {:EXIT, _pid, _reason} ->
          :gen_tcp.send(socket, "{\"ty\":\"test_exit\"}")
          :gen_tcp.close(socket)
      end
    end

    %__MODULE__{
      port: port,
      socket: socket,
      peer_ipc_path: peer_ipc_path,
    }
   end

  def send_packet(struct, msg) do
    msg_json = Jason.encode!(msg)
    :gen_tcp.send(struct.socket, msg_json)
  end

  def read_packet(state) do
    socket = state.socket

    :inet.setopts(socket, [{:active, :once}])

    receive do
      {:tcp, ^socket, msg} ->
        msg = Jason.decode!(msg)
        {:ok, msg}
      {:tcp_closed, ^socket} ->
        :closed
    end
  end

  def do_handshake(state) do
    key = LCOrch.KeyServer.get_key()

    uuid = UUID.uuid4()

    {:ok, msg} = read_packet(state)
    %{
      reply: reply,
      client_pubkey: client_pubkey,
      client_challenge: client_challenge,
    } = LCOrch.Node.Handshake.on_client_handshake(
      msg,
      uuid,
      key.prikey,
      key.pubkey
    )
    :ok = send_packet(state, reply)

    {:ok, msg} = read_packet(state)
    :ok = LCOrch.Node.Handshake.on_client_handshake_finish(
      msg,
      client_pubkey,
      client_challenge
    )

    state = %{
      state |
      uuid: uuid,
      pubkey: client_pubkey,
    }

    {:ok, state}
  end

  def close(struct) do
    ref = Port.monitor(struct.port)
    :gen_tcp.send(struct.socket, "{\"ty\":\"test_exit\"}")

    receive do
      {:DOWN, ^ref, :port, _object, _reason} ->
        :ok
    end
  end

end
