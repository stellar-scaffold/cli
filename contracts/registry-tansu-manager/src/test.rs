#![allow(clippy::needless_pass_by_value, clippy::should_panic_without_expect)]

extern crate std;

use soroban_sdk::{
    self, symbol_short, testutils::Address as _, vec, Address, Bytes, Env, IntoVal, String, Symbol,
    Vec,
};

use crate::{
    tansu_stub::{self, OutcomeContract, Proposal, ProposalStatus, Vote, VoteData},
    Error, RegistryTansuManager, RegistryTansuManagerClient,
};

// The `registry-stub` contract is wasm-imported via the soroban-sdk-tools macro
// (not `import_contract_client!`) so we get the test-only `AuthClient` builder
// alongside the regular `Client`. The AuthClient lets the
// `registry_rejects_direct_caller` test below express "outsider tries to call
// manager_only" with a single chained call instead of constructing MockAuth
// scaffolding by hand.
mod registry_stub {
    soroban_sdk_tools::contractimport!(file = "../../target/stellar/local/registry_stub.wasm");
}

// ---------------------------------------------------------------------------
// Test scaffolding
// ---------------------------------------------------------------------------

struct Setup {
    env: Env,
    project_key: Bytes,
    tansu: Address,
    registry: Address,
    #[allow(dead_code)]
    manager: Address,
    manager_client: RegistryTansuManagerClient<'static>,
}

fn setup() -> Setup {
    let env = Env::default();
    let project_key = Bytes::from_slice(&env, &[7u8; 16]);
    let tansu = env.register(tansu_stub::WASM, ());

    // Register the manager with a dummy registry address; the real address is
    // patched into instance storage below once the registry-stub is registered.
    let manager = env.register(
        RegistryTansuManager,
        (tansu.clone(), project_key.clone(), Address::generate(&env)),
    );
    let registry = env.register(registry_stub::WASM, (manager.clone(),));

    env.as_contract(&manager, || {
        crate::Storage::set_registry(&env, &registry);
    });

    let manager_client = RegistryTansuManagerClient::new(&env, &manager);

    Setup {
        env,
        project_key,
        tansu,
        registry,
        manager,
        manager_client,
    }
}

fn empty_vote_data(env: &Env) -> VoteData {
    VoteData {
        voting_ends_at: 0,
        public_voting: true,
        token_contract: None,
        votes: Vec::<Vote>::new(env),
    }
}

fn plant_proposal(
    env: &Env,
    tansu: &Address,
    project_key: &Bytes,
    id: u32,
    status: ProposalStatus,
    outcomes: Option<Vec<OutcomeContract>>,
) {
    let proposal = Proposal {
        id,
        title: String::from_str(env, "t"),
        proposer: Address::generate(env),
        ipfs: String::from_str(env, ""),
        vote_data: empty_vote_data(env),
        status,
        outcome_contracts: outcomes,
    };
    tansu_stub::Client::new(env, tansu).set_proposal(project_key, &proposal);
}

fn one_outcome(env: &Env, registry: &Address, value: u32) -> Vec<OutcomeContract> {
    vec![
        env,
        OutcomeContract {
            address: registry.clone(),
            execute_fn: symbol_short!("man_only"),
            args: vec![env, value.into_val(env)],
        },
    ]
}

// ---------------------------------------------------------------------------
// Happy path: approved proposal -> registry call succeeds via contract auth
// ---------------------------------------------------------------------------

#[test]
fn approved_proposal_forwards_to_registry() {
    let s = setup();
    let outcomes = vec![
        &s.env,
        OutcomeContract {
            address: s.registry.clone(),
            execute_fn: Symbol::new(&s.env, "manager_only"),
            args: vec![&s.env, 42u32.into_val(&s.env)],
        },
    ];
    plant_proposal(
        &s.env,
        &s.tansu,
        &s.project_key,
        1,
        ProposalStatus::Approved,
        Some(outcomes),
    );

    // No external signer needed: the manager contract's auth satisfies
    // registry's `manager.require_auth()` via the XCC contract-auth chain.
    let result: u32 = s
        .manager_client
        .execute(&1)
        .try_into()
        .expect("Val should decode to u32");

    assert_eq!(result, 42);

    let reg = registry_stub::Client::new(&s.env, &s.registry);
    assert_eq!(reg.recorded(), Some(42));
}

// ---------------------------------------------------------------------------
// Negative cases
// ---------------------------------------------------------------------------

#[test]
fn active_proposal_is_rejected() {
    let s = setup();
    plant_proposal(
        &s.env,
        &s.tansu,
        &s.project_key,
        1,
        ProposalStatus::Active,
        Some(one_outcome(&s.env, &s.registry, 1)),
    );

    let err = s.manager_client.try_execute(&1).err().unwrap().unwrap();
    assert_eq!(err, Error::NotApproved);
}

#[test]
fn rejected_proposal_is_rejected() {
    let s = setup();
    plant_proposal(
        &s.env,
        &s.tansu,
        &s.project_key,
        1,
        ProposalStatus::Rejected,
        Some(one_outcome(&s.env, &s.registry, 1)),
    );

    let err = s.manager_client.try_execute(&1).err().unwrap().unwrap();
    assert_eq!(err, Error::NotApproved);
}

#[test]
fn proposal_without_outcomes_is_rejected() {
    let s = setup();
    plant_proposal(
        &s.env,
        &s.tansu,
        &s.project_key,
        1,
        ProposalStatus::Approved,
        None,
    );

    let err = s.manager_client.try_execute(&1).err().unwrap().unwrap();
    assert_eq!(err, Error::NoOutcomeContracts);
}

#[test]
fn proposal_with_multiple_outcomes_is_rejected() {
    let s = setup();
    let outcomes = vec![
        &s.env,
        OutcomeContract {
            address: s.registry.clone(),
            execute_fn: Symbol::new(&s.env, "manager_only"),
            args: vec![&s.env, 1u32.into_val(&s.env)],
        },
        OutcomeContract {
            address: s.registry.clone(),
            execute_fn: Symbol::new(&s.env, "manager_only"),
            args: vec![&s.env, 2u32.into_val(&s.env)],
        },
    ];
    plant_proposal(
        &s.env,
        &s.tansu,
        &s.project_key,
        1,
        ProposalStatus::Approved,
        Some(outcomes),
    );

    let err = s.manager_client.try_execute(&1).err().unwrap().unwrap();
    assert_eq!(err, Error::MultipleOutcomes);
}

#[test]
fn proposal_targeting_wrong_address_is_rejected() {
    let s = setup();
    let wrong = Address::generate(&s.env);
    let outcomes = vec![
        &s.env,
        OutcomeContract {
            address: wrong,
            execute_fn: Symbol::new(&s.env, "manager_only"),
            args: vec![&s.env, 1u32.into_val(&s.env)],
        },
    ];
    plant_proposal(
        &s.env,
        &s.tansu,
        &s.project_key,
        1,
        ProposalStatus::Approved,
        Some(outcomes),
    );

    let err = s.manager_client.try_execute(&1).err().unwrap().unwrap();
    assert_eq!(err, Error::OutcomeTargetMismatch);
}

// ---------------------------------------------------------------------------
// Replay guard
// ---------------------------------------------------------------------------

#[test]
fn approved_proposal_cannot_be_replayed() {
    let s = setup();
    let outcomes = vec![
        &s.env,
        OutcomeContract {
            address: s.registry.clone(),
            execute_fn: Symbol::new(&s.env, "manager_only"),
            args: vec![&s.env, 7u32.into_val(&s.env)],
        },
    ];
    plant_proposal(
        &s.env,
        &s.tansu,
        &s.project_key,
        1,
        ProposalStatus::Approved,
        Some(outcomes),
    );

    s.manager_client.execute(&1);
    let err = s.manager_client.try_execute(&1).err().unwrap().unwrap();
    assert_eq!(err, Error::AlreadyExecuted);
}

#[test]
fn proposal_targeting_manager_itself_is_rejected() {
    // An attacker-crafted proposal whose outcome address is the manager
    // contract (not the registry) must be rejected — otherwise the manager
    // could be tricked into recursively re-entering itself.
    let s = setup();
    let outcomes = vec![
        &s.env,
        OutcomeContract {
            address: s.manager.clone(),
            execute_fn: Symbol::new(&s.env, "execute"),
            args: vec![&s.env, 1u32.into_val(&s.env)],
        },
    ];
    plant_proposal(
        &s.env,
        &s.tansu,
        &s.project_key,
        1,
        ProposalStatus::Approved,
        Some(outcomes),
    );

    let err = s.manager_client.try_execute(&1).err().unwrap().unwrap();
    assert_eq!(err, Error::OutcomeTargetMismatch);
}

// ---------------------------------------------------------------------------
// Auth-flow guard: the registry's manager-only function must reject calls
// that come from somewhere other than the manager contract.
// ---------------------------------------------------------------------------

#[test]
#[should_panic] // require_auth on the manager address fails for an outside caller
fn registry_rejects_direct_caller() {
    let s = setup();
    let outsider = Address::generate(&s.env);

    // Authorize the outsider (not the manager) and call manager_only directly.
    // AuthClient chains the mock-auth setup onto the call in one builder,
    // replacing the prior hand-built `setup_mock_auth(...)` + client.invoke().
    registry_stub::AuthClient::new(&s.env, &s.registry)
        .manager_only(&99u32)
        .authorize(&outsider)
        .invoke();
}
