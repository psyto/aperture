#!/usr/bin/env bash
# Submit an aperture range proof to the reactivated ZK ElGamal Proof Program on devnet via
# simulateTransaction (no fee charged, no signature needed). A funded/existing payer is required
# because the simulator loads the fee-payer account.
#
# Usage:  ./simulate.sh [payer_pubkey]
#   default payer = the throwaway devnet keypair in ../.devnet-payer.json; fund it first:
#     solana airdrop 1 $(solana-keygen pubkey ../.devnet-payer.json) -u devnet
#   or pass any funded devnet pubkey as the argument.
set -euo pipefail
PAYER="${1:-BjKr5GrbtX4saPvirVaW1QhjspaY9hgBFEzYvQaeRk1m}"
TX=$(cargo run --quiet -- "$PAYER")
curl -s -X POST https://api.devnet.solana.com -H 'Content-Type: application/json' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"simulateTransaction\",\"params\":[\"$TX\",{\"sigVerify\":false,\"replaceRecentBlockhash\":true,\"encoding\":\"base64\"}]}" \
  | python3 -c "import sys,json; v=json.load(sys.stdin)['result']['value']; print('err  :', v['err']); print('units:', v['unitsConsumed']); print('logs :'); [print('  ', l) for l in (v['logs'] or [])]"
# Success = "err: None" with non-zero units and a Verify... log = the live program accepted the proof.
