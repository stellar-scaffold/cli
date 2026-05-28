#!/usr/bin/env bash
# End-to-end test of registry-tansu-manager against the **live** Tansu DAO on
# Stellar testnet — exercises the full vote-then-execute cycle, no stub.
#
# Two-phase, because Tansu enforces a 24-hour minimum voting period (hardcoded
# `MIN_VOTING_PERIOD = 24*3600` in contract_dao.rs, no env override):
#
#   $ ./e2e-real-tansu-testnet.sh setup
#         -> registers a fresh Tansu project, deploys registry + manager,
#            uploads hello.wasm, creates a publish_hash proposal on Tansu,
#            votes yes, saves state to a sidecar file.
#         -> prints the exact follow-up command + timestamp.
#
#   $ ./e2e-real-tansu-testnet.sh finalize [state-file]   # ≥ 24h later
#         -> calls Tansu.execute (Active -> Approved),
#            calls manager.execute (forwards to registry.publish_hash),
#            verifies the published wasm hash on the registry.
#
# Live Tansu (testnet):
#   CBXKUSLQPVF35FYURR5C42BPYA5UOVDXX2ELKIM2CAJMCI6HXG2BHGZA
#   https://stellar.expert/explorer/testnet/contract/CBXKUSLQPVF35FYURR5C42BPYA5UOVDXX2ELKIM2CAJMCI6HXG2BHGZA
# Collateral token (testnet XLM via native SAC):
#   CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC
# Proposal collateral: 7 XLM (PROPOSAL_COLLATERAL); voting also takes 2 XLM
# per voter (VOTE_COLLATERAL). Both refunded on Tansu.execute.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
WASM_DIR="$REPO_ROOT/target/stellar/local"

NETWORK="${NETWORK:-testnet}"
TANSU_ID="${TANSU_ID:-CBXKUSLQPVF35FYURR5C42BPYA5UOVDXX2ELKIM2CAJMCI6HXG2BHGZA}"
HELLO_WASM="$WASM_DIR/hello.wasm"
REGISTRY_WASM="$WASM_DIR/registry.wasm"
MANAGER_WASM="$WASM_DIR/registry_tansu_manager.wasm"

usage() {
    cat <<EOF >&2
Usage: $0 <setup|finalize> [state-file]

  setup       Run phase 1: register Tansu project, deploy registry + manager,
              create + vote on proposal. Writes state to:
                $SCRIPT_DIR/e2e-real-tansu-state-<RUN_ID>.env
  finalize    Run phase 2 (after voting_ends_at):
                $0 finalize [path-to-state-file]
              If state-file is omitted, picks the most-recent state file in $SCRIPT_DIR.

Env:
  NETWORK         Stellar network alias (default: testnet)
  TANSU_ID        Tansu contract id (default: live testnet Tansu)
EOF
    exit 1
}

[[ $# -ge 1 ]] || usage
PHASE="$1"; shift || true

require_network() {
    if ! stellar network ls 2>/dev/null | grep -qx "$NETWORK"; then
        echo "❌ stellar network '$NETWORK' is not configured" >&2; exit 1
    fi
}

ensure_account() {
    local id="$1"
    if ! stellar keys ls 2>/dev/null | grep -qx "$id"; then
        echo "==> Generating + funding $id on $NETWORK"
        stellar keys generate --network "$NETWORK" --fund "$id" >/dev/null
    fi
}

invoke() { stellar contract invoke --network "$NETWORK" "$@"; }

# ---------------------------------------------------------------------------
# Phase 1: setup
# ---------------------------------------------------------------------------
phase_setup() {
    require_network
    for w in "$HELLO_WASM" "$REGISTRY_WASM" "$MANAGER_WASM"; do
        [[ -f "$w" ]] || { echo "❌ missing $w — run \`just build\` first" >&2; exit 1; }
    done

    RUN_ID="${RUN_ID:-$(date +%s)}"
    STATE_FILE="$SCRIPT_DIR/e2e-real-tansu-state-${RUN_ID}.env"
    HELLO_VERSION="${HELLO_VERSION:-0.1.0}"
    # Tansu enforces project name ≤ 15 chars. The name is also registered on
    # SorobanDomain under TLD .xlm, whose `validate_domain` requires bytes in
    # `[a-z]` only — no digits, no hyphens, no uppercase. Map run-id digits to
    # the a–j range so we keep determinism + uniqueness.
    if [[ -z "${PROJECT_NAME:-}" ]]; then
        short_id=$(printf '%s' "$RUN_ID" | tr '0-9' 'a-j')
        PROJECT_NAME="ee${short_id: -10}"  # 2 + 10 = 12 chars, all lowercase
    fi

    MAINTAINER_ID="${MAINTAINER_ID:-tansu-e2e-${RUN_ID}}"
    VOTER_ID="${VOTER_ID:-tansu-e2e-voter-${RUN_ID}}"
    ensure_account "$MAINTAINER_ID"
    ensure_account "$VOTER_ID"
    MAINTAINER_ADDR=$(stellar keys address "$MAINTAINER_ID")
    VOTER_ADDR=$(stellar keys address "$VOTER_ID")

    echo "==> Network:     $NETWORK"
    echo "==> Tansu:       $TANSU_ID"
    echo "==> Run id:      $RUN_ID"
    echo "==> Maintainer:  $MAINTAINER_ID ($MAINTAINER_ADDR)"
    echo "==> Voter:       $VOTER_ID ($VOTER_ADDR)"
    echo "==> State file:  $STATE_FILE"

    # 1. Register a fresh project on Tansu. The function returns the project_key
    #    (Bytes — keccak256(name) inside Tansu). We capture it for the manager
    #    constructor and proposal-target lookup.
    echo "==> Registering Tansu project '$PROJECT_NAME'"
    PROJECT_KEY_RAW=$(invoke --id "$TANSU_ID" --source "$MAINTAINER_ID" \
        --send=yes -- register \
        --maintainer "$MAINTAINER_ADDR" \
        --name "$PROJECT_NAME" \
        --maintainers "[\"$MAINTAINER_ADDR\"]" \
        --url "https://example.invalid/${PROJECT_NAME}" \
        --ipfs "QmExampleIpfs0000000000000000000000000000000000")
    # Strip quotes from the returned hex Bytes literal.
    PROJECT_KEY="${PROJECT_KEY_RAW//\"/}"
    echo "    project_key: $PROJECT_KEY"

    # 2. Add both maintainer and voter as Tansu members. Tansu auto-adds the
    #    proposer to the Abstain group on `create_proposal`, so the maintainer
    #    (proposer) can't be the one casting an Approve — we need a second
    #    account whose default vote weight of 1 is enough to carry a single-
    #    voter Approve over the proposer's Abstain.
    echo "==> Adding maintainer + voter as Tansu members"
    invoke --id "$TANSU_ID" --source "$MAINTAINER_ID" --send=yes \
        -- add_member \
        --member_address "$MAINTAINER_ADDR" \
        --meta "tansu-e2e maintainer" >/dev/null
    invoke --id "$TANSU_ID" --source "$VOTER_ID" --send=yes \
        -- add_member \
        --member_address "$VOTER_ADDR" \
        --meta "tansu-e2e voter" >/dev/null

    # 3. Upload hello.wasm to get the hash the proposal will register.
    echo "==> Uploading hello.wasm"
    HELLO_HASH=$(stellar contract upload --wasm "$HELLO_WASM" \
        --source "$MAINTAINER_ID" --network "$NETWORK")
    echo "    hash:        $HELLO_HASH"

    # 4. Deploy a fresh registry — admin & manager both set to the G account
    #    initially, so we can swap the manager to the manager contract before
    #    handing publishing power over to the DAO.
    echo "==> Deploying registry (admin=manager=$MAINTAINER_ID)"
    REGISTRY_ID=$(stellar contract deploy --wasm "$REGISTRY_WASM" \
        --source "$MAINTAINER_ID" --network "$NETWORK" \
        --alias "registry-tansu-e2e-${RUN_ID}" \
        -- --admin "$MAINTAINER_ADDR" --manager "\"$MAINTAINER_ADDR\"")
    echo "    registry:    $REGISTRY_ID"

    # 5. Deploy registry-tansu-manager, pointing at LIVE Tansu + our registry.
    echo "==> Deploying registry-tansu-manager"
    MANAGER_ID=$(stellar contract deploy --wasm "$MANAGER_WASM" \
        --source "$MAINTAINER_ID" --network "$NETWORK" \
        --alias "manager-tansu-e2e-${RUN_ID}" \
        -- \
        --tansu "$TANSU_ID" \
        --project_key "$PROJECT_KEY" \
        --registry "$REGISTRY_ID")
    echo "    manager:     $MANAGER_ID"

    # 6. Swap registry's manager to the manager contract. From here on, all
    #    manager-gated registry ops MUST come through Tansu proposals.
    echo "==> Installing manager contract on registry"
    invoke --id "$REGISTRY_ID" --source "$MAINTAINER_ID" --send=yes \
        -- set_manager --new_manager "$MANAGER_ID" >/dev/null

    # 6b. Hand Tansu maintainership over to the manager. After this, when the
    #     finalize phase calls `manager.trigger(proposal_id)`, the manager is
    #     the direct caller of Tansu.execute, so Tansu's internal
    #     `maintainer.require_auth` is satisfied by contract-implicit auth
    #     (no auth entry needed, no non-root recording issue).
    echo "==> Tansu.update_config — replace maintainers with [$MANAGER_ID]"
    invoke --id "$TANSU_ID" --source "$MAINTAINER_ID" --send=yes \
        -- update_config \
        --maintainer "$MAINTAINER_ADDR" \
        --key "$PROJECT_KEY" \
        --maintainers "[\"$MANAGER_ID\"]" \
        --url "https://example.invalid/${PROJECT_NAME}" \
        --ipfs "QmExampleIpfs0000000000000000000000000000000000" >/dev/null

    # 7. Build the proposal. Outcome targets registry.publish_hash directly —
    #    the manager pre-authorizes this specific call via
    #    `authorize_as_current_contract` inside `trigger` so the registry's
    #    `manager.require_auth` is satisfied.
    NOW=$(date +%s)
    VOTING_ENDS_AT=$((NOW + 24*3600 + 600))   # 24h + 10min cushion
    PROPOSAL_TITLE="${PROPOSAL_TITLE:-Add hello@${HELLO_VERSION} to registry}"
    OUTCOME=$(cat <<EOF
[{
  "address": "$REGISTRY_ID",
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

    echo "==> Creating Tansu proposal (voting_ends_at=$VOTING_ENDS_AT)"
    PROPOSAL_ID_RAW=$(invoke --id "$TANSU_ID" --source "$MAINTAINER_ID" --send=yes \
        -- create_proposal \
        --proposer "$MAINTAINER_ADDR" \
        --project_key "$PROJECT_KEY" \
        --title "$PROPOSAL_TITLE" \
        --ipfs "QmExampleIpfs1111111111111111111111111111111111" \
        --voting_ends_at "$VOTING_ENDS_AT" \
        --public_voting true \
        --outcome_contracts "$OUTCOME")
    PROPOSAL_ID="${PROPOSAL_ID_RAW//\"/}"
    echo "    proposal_id: $PROPOSAL_ID"

    # 8. Vote Approve as the second account. The proposer was auto-added to
    #    Abstain when `create_proposal` ran; this Approve out-votes that.
    echo "==> Voting Approve as $VOTER_ID"
    VOTE_PAYLOAD=$(cat <<EOF
{"PublicVote":{
  "address":"$VOTER_ADDR",
  "weight":1,
  "vote_choice":"Approve"
}}
EOF
)
    invoke --id "$TANSU_ID" --source "$VOTER_ID" --send=yes \
        -- vote \
        --voter "$VOTER_ADDR" \
        --project_key "$PROJECT_KEY" \
        --proposal_id "$PROPOSAL_ID" \
        --vote "$VOTE_PAYLOAD" >/dev/null

    # 9. Save state for the finalize phase.
    cat > "$STATE_FILE" <<EOF
# Created by e2e-real-tansu-testnet.sh setup at $(date -u +%FT%TZ)
NETWORK=$NETWORK
TANSU_ID=$TANSU_ID
MAINTAINER_ID=$MAINTAINER_ID
MAINTAINER_ADDR=$MAINTAINER_ADDR
VOTER_ID=$VOTER_ID
VOTER_ADDR=$VOTER_ADDR
RUN_ID=$RUN_ID
PROJECT_NAME=$PROJECT_NAME
PROJECT_KEY=$PROJECT_KEY
REGISTRY_ID=$REGISTRY_ID
MANAGER_ID=$MANAGER_ID
PROPOSAL_ID=$PROPOSAL_ID
VOTING_ENDS_AT=$VOTING_ENDS_AT
HELLO_VERSION=$HELLO_VERSION
HELLO_HASH=$HELLO_HASH
EOF
    chmod 0644 "$STATE_FILE"

    local remain_h=$(( (VOTING_ENDS_AT - NOW) / 3600 ))
    local remain_m=$(( (VOTING_ENDS_AT - NOW) % 3600 / 60 ))
    cat <<EOF

✅ Phase 1 (setup) complete.

   State saved to: $STATE_FILE
   Vote closes at: $(date -u -d "@$VOTING_ENDS_AT" +%FT%TZ) (${remain_h}h ${remain_m}m from now)

   Resume with:
      $0 finalize $STATE_FILE
EOF
}

# ---------------------------------------------------------------------------
# Phase 2: finalize
# ---------------------------------------------------------------------------
phase_finalize() {
    require_network

    local state_file="${1:-}"
    if [[ -z "$state_file" ]]; then
        state_file=$(ls -t "$SCRIPT_DIR"/e2e-real-tansu-state-*.env 2>/dev/null | head -n1 || true)
    fi
    [[ -n "$state_file" && -f "$state_file" ]] || {
        echo "❌ no state file (looked in $SCRIPT_DIR/e2e-real-tansu-state-*.env)" >&2; exit 1
    }
    # shellcheck source=/dev/null
    source "$state_file"
    echo "==> State file: $state_file"
    echo "==> Run id:     $RUN_ID"

    NOW=$(date +%s)
    if (( NOW < VOTING_ENDS_AT )); then
        echo "❌ Voting period hasn't ended yet. Earliest: $(date -u -d "@$VOTING_ENDS_AT" +%FT%TZ) (in $(( (VOTING_ENDS_AT - NOW) / 60 ))m)" >&2
        exit 1
    fi

    # 1. manager.trigger drives Tansu.execute + the publish in one tx. The
    #    manager (set as Tansu maintainer in setup step 6b) is the direct
    #    caller of Tansu.execute, satisfying Tansu's
    #    `maintainer.require_auth`. The manager pre-authorizes the registry
    #    publish via `authorize_as_current_contract`, satisfying the
    #    registry's `manager.require_auth`. Single tx.
    echo "==> manager.trigger -> Tansu.execute -> registry.publish_hash (single tx)"
    invoke --id "$MANAGER_ID" --source "$MAINTAINER_ID" --send=yes \
        -- trigger --proposal_id "$PROPOSAL_ID" >/dev/null

    # 2. Verify the registry now has hello@version pointing at our uploaded hash.
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

    # 3. Replay guard — Tansu's own ProposalActive check.
    echo "==> Replay check — second manager.trigger must fail (ProposalActive)"
    REPLAY_OUT=$(invoke --id "$MANAGER_ID" --source "$MAINTAINER_ID" --send=yes \
        -- trigger --proposal_id "$PROPOSAL_ID" 2>&1 || true)
    if grep -qE 'ProposalActive|Error\(Contract, ?#402\)' <<<"$REPLAY_OUT"; then
        echo "    ✓ replay rejected by Tansu"
    else
        echo "    ❌ replay was NOT rejected" >&2
        echo "$REPLAY_OUT" >&2
        exit 1
    fi

    cat <<EOF

✅ Real-Tansu E2E pass
   tansu:    $TANSU_ID
   project:  $PROJECT_NAME ($PROJECT_KEY)
   registry: $REGISTRY_ID
   manager:  $MANAGER_ID
   proposal: #$PROPOSAL_ID -> Approved (via manager.trigger)
   hello:    $HELLO_HASH @ $HELLO_VERSION
EOF
}

case "$PHASE" in
    setup)    phase_setup    "$@" ;;
    finalize) phase_finalize "$@" ;;
    *)        usage ;;
esac
