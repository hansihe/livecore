defmodule LCOrch.Crypto do

  def ecdsa_sig_rs_to_asn1(rs) when byte_size(rs) == 64 do
    <<sig_r::unsigned-integer-size(256), sig_s::unsigned-integer-size(256)>> = rs
    :public_key.der_encode(:"ECDSA-Sig-Value", {:"ECDSA-Sig-Value", sig_r, sig_s})
  end

  def ecdsa_sig_asn1_to_rs(asn1) do
    {:"ECDSA-Sig-Value", sig_r, sig_s} = :public_key.der_decode(:"ECDSA-Sig-Value", asn1)
    <<sig_r::unsigned-integer-size(256), sig_s::unsigned-integer-size(256)>>
  end

  def gen_ecdsa_keypair() do
    :crypto.generate_key(:ecdh, :secp256r1)
  end

  def ecdsa_verify(message, signature, pubkey) do
    asn_signature = ecdsa_sig_rs_to_asn1(signature)
    :crypto.verify(:ecdsa, :sha256, message, asn_signature, [pubkey, :secp256r1])
  end

  def ecdsa_sign(message, prikey) do
    signature = :crypto.sign(:ecdsa, :sha256, message, [prikey, :secp256r1])
    ecdsa_sig_asn1_to_rs(signature)
  end

end
