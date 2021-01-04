#defmodule LCOrch.TestUtils.IPCAcceptor do
#  @moduledoc """
#  IPC socket acceptor, mostly for use in test suite.
#  """
#
#  def start_link(socket_path) do
#    GenServer.start_link(__MODULE__, nil, name: __MODULE__)
#
#
#    #spawn_link fn ->
#    #  listen(socket_path)
#    #end
#  end
#
#  defstruct socket_name: nil, socket: nil
#
#  def init(nil) do
#    socket_path = "/tmp/" <> UUID.uuid4()
#
#    args = [
#      :binary,
#      # 4 byte length prefixed packets
#      {:packet, 4},
#      {:ifaddr, {:local, socket_path}},
#      {:active, false},
#    ]
#    {:ok, socket} = :gen_tcp.listen(0, args)
#
#    state = %__MODULE__{
#      socket_path: socket_path,
#      socket: socket,
#    }
#
#    {:ok, state}
#  end
#
#  def listen(socket_path) do
#    case File.lstat(socket_path) do
#      {:error, :enoent} ->
#        nil
#      {:ok, %{type: :other}} ->
#        File.rm!(socket_path)
#    end
#
#    args = [
#      :binary,
#      # 4 byte length prefixed packets
#      {:packet, 4},
#      {:ifaddr, {:local, socket_path}},
#      {:active, false},
#    ]
#    {:ok, port} = :gen_tcp.listen(0, args)
#    accept(port)
#  end
#
#  def accept(listen_socket) do
#    {:ok, socket} = :gen_tcp.accept(listen_socket)
#    pid = spawn_link fn ->
#      handle(socket)
#    end
#    :gen_tcp.controlling_process(socket, pid)
#    accept(listen_socket)
#  end
#
#  def handle(socket) do
#    self_pid = self()
#
#    spec = %{id: LCOrch.Node, start: {LCOrch.Node, :start_link, [self_pid]}}
#    {:ok, node_server} = DynamicSupervisor.start_child(LCOrch.NodeSupervisor, spec)
#
#    :inet.setopts(socket, [{:active, :once}])
#
#    loop(socket, node_server)
#  end
#
#  def loop(socket, node_server) do
#    receive do
#      {:tcp, ^socket, data} ->
#        :inet.setopts(socket, [{:active, :once}])
#
#        msg = Jason.decode!(data)
#        case GenServer.call(node_server, {:socket_message, msg}) do
#          {:reply, msg} ->
#            msg_json = Jason.encode!(msg)
#            :gen_tcp.send(socket, msg_json)
#            :ok
#          :ok ->
#            :ok
#        end
#
#        loop(socket, node_server)
#      {:tcp_closed, ^socket} ->
#        IO.inspect {:tcp_closed, socket}
#        :ok
#      msg -> IO.inspect {:wtf_msg, msg}
#    end
#  end
#
#end
