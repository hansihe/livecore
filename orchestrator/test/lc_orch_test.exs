defmodule LCOrchTest do
  use ExUnit.Case
  doctest LCOrch

  alias LCOrch.TestUtils.NodeSpawn

  test "greets the world" do
    assert LCOrch.hello() == :world
  end

  test "peers can connect" do
    node1 = NodeSpawn.spawn()
    {:ok, node1} = NodeSpawn.do_handshake(node1)
    node1_uuid = node1.uuid

    node2 = NodeSpawn.spawn()
    {:ok, node2} = NodeSpawn.do_handshake(node2)
    node2_uuid = node2.uuid

    nonce1 = UUID.uuid4()
    nonce2 = UUID.uuid4()

    msg = %{
      "ty" => "connect_peer",
      "connector" => %{
        "ty" => "ipc_server",
      },
      "peer_uuid" => node2.uuid,
      "peer_pubkey" => :erlang.binary_to_list(node2.pubkey),
      "self_nonce" => nonce1,
      "peer_nonce" => nonce2,
    }
    :ok = NodeSpawn.send_packet(node1, msg)

    msg = %{
      "ty" => "connect_peer",
      "connector" => %{
        "ty" => "ipc_client",
        "socket_path" => node1.peer_ipc_path,
      },
      "peer_uuid" => node1.uuid,
      "peer_pubkey" => :erlang.binary_to_list(node1.pubkey),
      "self_nonce" => nonce2,
      "peer_nonce" => nonce1,
    }
    :ok = NodeSpawn.send_packet(node2, msg)

    {:ok, %{"ty" => "peer_connection_success", "peer_uuid" => ^node2_uuid}} =
      NodeSpawn.read_packet(node1)

    {:ok, %{"ty" => "peer_connection_success", "peer_uuid" => ^node1_uuid}} =
      NodeSpawn.read_packet(node2)

    msg = %{
      "ty" => "object_manifest",
      "hash" => "0000000000000000000000000000000000000000000000000000000000000000",
      "tags" => [],
      "size" => 1000,
      "fragment_size" => 9,
      "fragments" => [
        %{
          "hash" => "0000000000000000000000000000000000000000000000000000000000000000",
        },
        %{
          "hash" => "0000000000000000000000000000000000000000000000000000000000000000",
        },
      ],
    }
    :ok = NodeSpawn.send_packet(node1, msg)

    :ok = NodeSpawn.close(node1)
    :ok = NodeSpawn.close(node2)
  end
end
