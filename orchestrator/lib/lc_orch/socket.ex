defmodule LCOrch.Socket do
  @behaviour Phoenix.Socket.Transport

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

  def init(state) do
    IO.inspect {:init, state}
    {:ok, state}
  end

  def handle_in({text, _opts}, state) do
    IO.inspect {:message, text}
    {:reply, :ok, {:text, text}, state}
  end

  #def handle_info(_, state) do
  #  {}
  #end

  def terminate(reason, _state) do
    IO.inspect {:terminate, reason}
    :ok
  end

end
