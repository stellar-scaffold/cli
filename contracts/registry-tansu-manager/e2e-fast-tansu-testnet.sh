#!/usr/bin/env bash
# Fast e2e against a custom-built Tansu that exposes per-project
# `min_voting_period` AND `execute_delay` on `register(...)`. Lets us exercise
# the same flow as e2e-real-tansu-testnet.sh in ~2 minutes instead of 48+ hours
# (stock Tansu has 24h MIN_VOTING_PERIOD + 24h TIMELOCK_DELAY).
#
# Custom Tansu (testnet): CDK7JBIIP6E75HOYLGRGWAHQLT6JUNUXQ7GNOYS3NAP26GISUXJ26UON
#   https://stellar.expert/explorer/testnet/contract/CDK7JBIIP6E75HOYLGRGWAHQLT6JUNUXQ7GNOYS3NAP26GISUXJ26UON
#
# Flow (single phase):
#   1.  Register a fresh Tansu project with min_voting_period=$MIN_VOTING_PERIOD seconds
#   2.  Add maintainer + voter as Tansu members
#   3.  Upload hello.wasm, deploy registry, deploy manager, set_manager
#   4.  Create proposal whose outcome targets `manager.publish_hash(...)` (a
#       no-op proxy on the manager, same signature as the registry's). Tansu's
#       auto-invocation in `contract_dao.rs::execute` lands on this no-op
#       instead of the registry — pointing the outcome at the registry
#       directly would fail at the registry's `manager.require_auth` (Tansu
#       isn't in that auth chain) and `try_invoke_contract` would propagate
#       the failure, reverting the whole Tansu tx.
#   5.  Vote Approve from the second account
#   6.  Sleep until past voting_ends_at + execute_delay
#   7.  Tansu.execute (Active -> Approved). The no-op proxy succeeds; the
#       proposal status persists.
#   8.  manager.execute(proposal_id) — re-reads the now-Approved proposal,
#       checks the outcome targets one of its proxies (and isn't a recursive
#       `execute` re-entry), then forwards `oc.execute_fn + oc.args` to the
#       registry with this contract's auth satisfying manager.require_auth.
#   9.  Assert registry.fetch_hash returns the wasm hash we uploaded
#  10.  Replay guard via second manager.execute
#
# Env (all optional):
#   NETWORK              Stellar network alias (default: testnet)
#   TANSU_ID             Tansu contract id (default: custom-built testnet Tansu above)
#   MIN_VOTING_PERIOD    Seconds. Default 60. Must be ≥ ~30s to leave room for tx propagation.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
WASM_DIR="$REPO_ROOT/target/stellar/local"

NETWORK="${NETWORK:-testnet}"
TANSU_ID="${TANSU_ID:-CDK7JBIIP6E75HOYLGRGWAHQLT6JUNUXQ7GNOYS3NAP26GISUXJ26UON}"
MIN_VOTING_PERIOD="${MIN_VOTING_PERIOD:-60}"
# Seconds between voting_ends_at and when Tansu.execute is callable. The custom
# Tansu rejects 0 (InvalidVotingPeriod / #212) — any positive value is fine.
EXECUTE_DELAY="${EXECUTE_DELAY:-60}"
HELLO_VERSION="${HELLO_VERSION:-0.1.0}"
RUN_ID="${RUN_ID:-$(date +%s)}"

HELLO_WASM="$WASM_DIR/hello.wasm"
REGISTRY_WASM="$WASM_DIR/registry.wasm"
MANAGER_WASM="$WASM_DIR/registry_tansu_manager.wasm"

for w in "$HELLO_WASM" "$REGISTRY_WASM" "$MANAGER_WASM"; do
    [[ -f "$w" ]] || { echo "❌ missing $w — run \`just build\` first" >&2; exit 1; }
done

if ! stellar network ls 2>/dev/null | grep -qx "$NETWORK"; then
    echo "❌ stellar network '$NETWORK' is not configured" >&2; exit 1
fi

ensure_account() {
    local id="$1"
    if ! stellar keys ls 2>/dev/null | grep -qx "$id"; then
        echo "==> Generating + funding $id on $NETWORK"
        stellar keys generate --network "$NETWORK" --fund "$id" >/dev/null
    fi
}

invoke() { stellar contract invoke --network "$NETWORK" "$@"; }

MAINTAINER_ID="${MAINTAINER_ID:-tansu-fast-${RUN_ID}}"
VOTER_ID="${VOTER_ID:-tansu-fast-voter-${RUN_ID}}"
ensure_account "$MAINTAINER_ID"
ensure_account "$VOTER_ID"
MAINTAINER_ADDR=$(stellar keys address "$MAINTAINER_ID")
VOTER_ADDR=$(stellar keys address "$VOTER_ID")

# Tansu name validation (SorobanDomain): ≤15 chars, [a-z] only. Map run-id digits → a-j.
short_id=$(printf '%s' "$RUN_ID" | tr '0-9' 'a-j')
PROJECT_NAME="${PROJECT_NAME:-ff${short_id: -10}}"

echo "==> Network:            $NETWORK"
echo "==> Tansu (custom):     $TANSU_ID"
echo "==> Run id:             $RUN_ID"
echo "==> Maintainer:         $MAINTAINER_ID ($MAINTAINER_ADDR)"
echo "==> Voter:              $VOTER_ID ($VOTER_ADDR)"
echo "==> min_voting_period:  ${MIN_VOTING_PERIOD}s"
echo "==> execute_delay:      ${EXECUTE_DELAY}s"

# 1. Register project with short min_voting_period + execute_delay.
echo "==> Registering project '$PROJECT_NAME'"
PROJECT_KEY_RAW=$(invoke --id "$TANSU_ID" --source "$MAINTAINER_ID" --send=yes \
    -- register \
    --maintainer "$MAINTAINER_ADDR" \
    --name "$PROJECT_NAME" \
    --maintainers "[\"$MAINTAINER_ADDR\"]" \
    --url "https://example.invalid/${PROJECT_NAME}" \
    --ipfs "QmExampleIpfs0000000000000000000000000000000000" \
    --min_voting_period "$MIN_VOTING_PERIOD" \
    --execute_delay "$EXECUTE_DELAY")
PROJECT_KEY="${PROJECT_KEY_RAW//\"/}"
echo "    project_key:        $PROJECT_KEY"

# Sanity-check: confirm both per-project knobs took.
ACTUAL_MVP=$(invoke --id "$TANSU_ID" --source "$MAINTAINER_ID" \
    -- get_min_voting_period --project_key "$PROJECT_KEY")
ACTUAL_EXD=$(invoke --id "$TANSU_ID" --source "$MAINTAINER_ID" \
    -- get_execute_delay --project_key "$PROJECT_KEY")
echo "    confirmed:          min_voting_period=${ACTUAL_MVP}s execute_delay=${ACTUAL_EXD}s"

# 2. Members. Already-existing members from a prior run will trip MemberAlreadyExist;
#    ignore that case so this script can be re-run with sticky $MAINTAINER_ID/$VOTER_ID.
echo "==> Adding maintainer + voter as members"
for who in "$MAINTAINER_ID:$MAINTAINER_ADDR:maintainer" "$VOTER_ID:$VOTER_ADDR:voter"; do
    IFS=: read -r src addr role <<<"$who"
    out=$(invoke --id "$TANSU_ID" --source "$src" --send=yes \
        -- add_member --member_address "$addr" --meta "tansu-fast $role" 2>&1 || true)
    if grep -q "MemberAlreadyExist\|#205" <<<"$out"; then
        echo "    $role $addr: already a member (ok)"
    elif grep -q "✅ Transaction submitted successfully" <<<"$out"; then
        echo "    $role $addr: added"
    else
        echo "$out" >&2
        echo "❌ add_member failed for $role" >&2
        exit 1
    fi
done

# 3. Upload hello.wasm.
echo "==> Uploading hello.wasm"
HELLO_HASH=$(stellar contract upload --wasm "$HELLO_WASM" \
    --source "$MAINTAINER_ID" --network "$NETWORK")
echo "    hash:               $HELLO_HASH"

# 4. Deploy registry (admin=manager=$MAINTAINER initially) and the manager contract.
echo "==> Deploying registry"
REGISTRY_ID=$(stellar contract deploy --wasm "$REGISTRY_WASM" \
    --source "$MAINTAINER_ID" --network "$NETWORK" \
    --alias "registry-tansu-fast-${RUN_ID}" \
    -- --admin "$MAINTAINER_ADDR" --manager "\"$MAINTAINER_ADDR\"")
echo "    registry:           $REGISTRY_ID"

echo "==> Deploying registry-tansu-manager"
MANAGER_ID=$(stellar contract deploy --wasm "$MANAGER_WASM" \
    --source "$MAINTAINER_ID" --network "$NETWORK" \
    --alias "manager-tansu-fast-${RUN_ID}" \
    -- \
    --tansu "$TANSU_ID" \
    --project_key "$PROJECT_KEY" \
    --registry "$REGISTRY_ID")
echo "    manager:            $MANAGER_ID"

echo "==> Installing manager contract on registry"
invoke --id "$REGISTRY_ID" --source "$MAINTAINER_ID" --send=yes \
    -- set_manager --new_manager "$MANAGER_ID" >/dev/null

# 5. Create proposal whose outcome targets registry.publish_hash(hello).
NOW=$(date +%s)
VOTING_ENDS_AT=$((NOW + MIN_VOTING_PERIOD + 15))   # +15s buffer for tx propagation
echo "==> Creating proposal (voting_ends_at=$VOTING_ENDS_AT, in ~$((VOTING_ENDS_AT-NOW))s)"
# Outcome targets the manager's no-op `publish_hash` proxy (same signature as
# the registry's). Tansu's auto-invocation lands there harmlessly so the
# proposal can flip to Approved; manager.execute(proposal_id) then re-reads
# the same outcome and forwards `publish_hash + args` to the registry with
# this contract's auth.
OUTCOME=$(cat <<EOF
[{
  "address": "$MANAGER_ID",
  "execute_fn": "publish_hash",
  "args": [
    {"string": "hello"},
    {"address": "$MAINTAINER_ADDR"},
    {"bytes": "$HELLO_HASH"},
    {"string": "$HELLO_VERSION"}
  ]
}]
EOF
)
PROPOSAL_ID_RAW=$(invoke --id "$TANSU_ID" --source "$MAINTAINER_ID" --send=yes \
    -- create_proposal \
    --proposer "$MAINTAINER_ADDR" \
    --project_key "$PROJECT_KEY" \
    --title "Add hello@${HELLO_VERSION} to registry (fast)" \
    --ipfs "QmExampleIpfs1111111111111111111111111111111111" \
    --voting_ends_at "$VOTING_ENDS_AT" \
    --public_voting true \
    --outcome_contracts "$OUTCOME")
PROPOSAL_ID="${PROPOSAL_ID_RAW//\"/}"
echo "    proposal_id:        $PROPOSAL_ID"

# 6. Vote Approve from the voter (proposer was auto-Abstained).
echo "==> Voting Approve as voter"
VOTE_PAYLOAD=$(cat <<EOF
{"PublicVote":{"address":"$VOTER_ADDR","weight":1,"vote_choice":"Approve"}}
EOF
)
invoke --id "$TANSU_ID" --source "$VOTER_ID" --send=yes \
    -- vote \
    --voter "$VOTER_ADDR" \
    --project_key "$PROJECT_KEY" \
    --proposal_id "$PROPOSAL_ID" \
    --vote "$VOTE_PAYLOAD" >/dev/null

# 7. Wait until voting_ends_at + execute_delay + a slack for ledger time lag.
WAIT_UNTIL=$((VOTING_ENDS_AT + EXECUTE_DELAY + 20))
while (( $(date +%s) < WAIT_UNTIL )); do
    remain=$((WAIT_UNTIL - $(date +%s)))
    printf "\r==> Waiting for voting period + execute_delay (%ds remaining)... " "$remain"
    sleep 5
done
echo ""

# 8. Tansu.execute moves the proposal from Active to Approved.
echo "==> Tansu.execute (Active -> Approved)"
STATUS_RAW=$(invoke --id "$TANSU_ID" --source "$MAINTAINER_ID" --send=yes \
    -- execute \
    --maintainer "$MAINTAINER_ADDR" \
    --project_key "$PROJECT_KEY" \
    --proposal_id "$PROPOSAL_ID")
STATUS="${STATUS_RAW//\"/}"
echo "    status:             $STATUS"
[[ "$STATUS" == "Approved" ]] || { echo "❌ proposal didn't pass: $STATUS" >&2; exit 1; }

# 9. manager.execute -> registry.publish_hash via XCC. Manager re-reads the
#    Approved proposal from Tansu, verifies the outcome targets one of its
#    no-op proxies (not `execute` itself), and forwards execute_fn + args to
#    the registry.
echo "==> manager.execute -> registry.publish_hash"
invoke --id "$MANAGER_ID" --source "$MAINTAINER_ID" --send=yes \
    -- execute --proposal_id "$PROPOSAL_ID" >/dev/null

# 10. Verify the publish landed.
echo "==> Verifying registry has hello@$HELLO_VERSION -> $HELLO_HASH"
PUBLISHED_HASH_RAW=$(invoke --id "$REGISTRY_ID" --source "$MAINTAINER_ID" \
    -- fetch_hash --wasm_name hello --version "\"$HELLO_VERSION\"")
PUBLISHED_HASH="${PUBLISHED_HASH_RAW//\"/}"
if [[ "$PUBLISHED_HASH" == "$HELLO_HASH" ]]; then
    echo "    ✓ registry resolved hello@$HELLO_VERSION -> $PUBLISHED_HASH"
else
    echo "    ❌ registry returned $PUBLISHED_HASH, expected $HELLO_HASH" >&2
    exit 1
fi

# 11. Replay guard.
echo "==> Replay check — second manager.execute must fail with AlreadyExecuted"
REPLAY_OUT=$(invoke --id "$MANAGER_ID" --source "$MAINTAINER_ID" --send=yes \
    -- execute --proposal_id "$PROPOSAL_ID" 2>&1 || true)
if grep -qE 'AlreadyExecuted|Error\(Contract, ?#5\)' <<<"$REPLAY_OUT"; then
    echo "    ✓ replay rejected"
else
    echo "    ❌ replay was NOT rejected" >&2
    echo "$REPLAY_OUT" >&2
    exit 1
fi

cat <<EOF

✅ Fast real-Tansu E2E pass
   tansu:    $TANSU_ID
   project:  $PROJECT_NAME ($PROJECT_KEY)
   period:   ${MIN_VOTING_PERIOD}s
   registry: $REGISTRY_ID
   manager:  $MANAGER_ID
   proposal: #$PROPOSAL_ID -> $STATUS
   hello:    $HELLO_HASH @ $HELLO_VERSION
EOF
