defmodule LCOrch.Node.Handshake do

  alias LCOrch.Crypto

  def on_client_handshake(
    msg = %{"ty" => "client_handshake", "version" => 0},
    client_uuid,
    server_prikey,
    server_pubkey
  ) do
    client_pubkey = :erlang.list_to_binary(msg["pubkey"])

    challenge = :erlang.list_to_binary(msg["challenge"]["challenge"])
    challenge_response = sign_challenge_response(server_prikey, challenge)

    client_challenge = :crypto.strong_rand_bytes(32)

    reply = %{
      "ty" => "server_handshake",
      "client_uuid" => client_uuid,
      "pubkey" => :erlang.binary_to_list(server_pubkey),
      "challenge_response" => challenge_response,
      "challenge" => %{
        "challenge" => :erlang.binary_to_list(client_challenge),
      }
    }

    %{
      reply: reply,
      client_pubkey: client_pubkey,
      client_challenge: client_challenge,
    }
  end

  def on_client_handshake_finish(
    msg = %{"ty" => "client_handshake_finish"},
    client_pubkey,
    client_challenge
  ) do
    true = validate_challenge_response(
      client_pubkey,
      client_challenge,
      :erlang.list_to_binary(msg["challenge_response"]["challenge_response"]),
      :erlang.list_to_binary(msg["challenge_response"]["signature"])
    )

    :ok
  end

  defp sign_challenge_response(prikey, challenge) when byte_size(challenge) == 32 do
    nonce = :crypto.strong_rand_bytes(32)
    challenge_response = "__HANDSHAKE_CHALLENGE__" <> challenge <> nonce <> "__HANDSHAKE_CHALLENGE__"
    challenge_response_sign = Crypto.ecdsa_sign(challenge_response, prikey)

    %{
      "challenge_response" => :erlang.binary_to_list(challenge_response),
      "signature" => :erlang.binary_to_list(challenge_response_sign),
    }
  end

  defp validate_challenge_response(pubkey, challenge, response, signature) do
    <<"__HANDSHAKE_CHALLENGE__", ^challenge::bytes-size(32), _::bytes-size(32), "__HANDSHAKE_CHALLENGE__">> = response
    Crypto.ecdsa_verify(response, signature, pubkey)
  end

end
