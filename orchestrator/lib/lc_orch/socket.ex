defmodule LCOrch.Socket do
  @behaviour Phoenix.Socket.Transport

  defstruct [node_server: nil]

  def child_spec(opts) do
    fun = fn ->
      :ok
    end

    %{id: Task, start: {Task, :start_link, [fun]}, restart: :transient}
  end

  def connect(map) do
    IO.inspect {:connect, map}
    {:ok, map}
  end

  def init(_map) do
    self_pid = self()

    spec = %{id: LCOrch.Node, start: {LCOrch.Node, :start_link, [self_pid]}}
    {:ok, node_server} = DynamicSupervisor.start_child(LCOrch.NodeSupervisor, spec)

    state = %__MODULE__{
      node_server: node_server,
    }

    {:ok, state}
  end

  def handle_in({text, _opts}, state) do
    msg = Jason.decode!(text)
    case GenServer.call(state.node_server, {:socket_message, msg}) do
      {:reply, msg} ->
        msg_json = Jason.encode!(msg)
        {:reply, :ok, {:text, msg_json}, state}
      :ok ->
        {:ok, state}
    end
  end

  #def handle_info(_, state) do
  #  {}
  #end

  def terminate(reason, _state) do
    IO.inspect {:terminate, reason}
    :ok
  end

end
