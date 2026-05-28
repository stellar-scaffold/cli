#![no_std]
#![allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]

use soroban_sdk::{
    contract, contractimpl, contracttype, vec, Address, Bytes, BytesN, Env, IntoVal, String,
    Symbol, Val, Vec,
};

// Tansu Proposal types — kept in lock-step with both `Consulting-Manao/tansu`
// `contracts/tansu/src/types.rs` and `contracts/registry-tansu-manager/src/lib.rs`.
// Duplicated here (rather than path-dep'd) because linking the manager crate as
// an `rlib` would re-export its `#[contractimpl]` functions into this stub's
// wasm.

#[contracttype]
#[derive(Clone)]
pub enum VoteChoice {
    Approve,
    Reject,
    Abstain,
}

#[contracttype]
#[derive(Clone)]
pub struct PublicVote {
    pub address: Address,
    pub weight: u32,
    pub vote_choice: VoteChoice,
}

#[contracttype]
#[derive(Clone)]
pub struct AnonymousVote {
    pub address: Address,
    pub weight: u32,
    pub encrypted_seeds: Vec<String>,
    pub encrypted_votes: Vec<String>,
    pub commitments: Vec<BytesN<96>>,
}

#[contracttype]
#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Vote {
    PublicVote(PublicVote),
    AnonymousVote(AnonymousVote),
}

#[contracttype]
#[derive(Clone)]
pub struct VoteData {
    pub voting_ends_at: u64,
    pub public_voting: bool,
    pub token_contract: Option<Address>,
    pub votes: Vec<Vote>,
}

#[contracttype]
#[derive(Clone)]
pub enum ProposalStatus {
    Active,
    Approved,
    Rejected,
    Cancelled,
    Malicious,
}

#[contracttype]
#[derive(Clone)]
pub struct OutcomeContract {
    pub address: Address,
    pub execute_fn: Symbol,
    pub args: Vec<Val>,
}

#[contracttype]
#[derive(Clone)]
pub struct Proposal {
    pub id: u32,
    pub title: String,
    pub proposer: Address,
    pub ipfs: String,
    pub vote_data: VoteData,
    pub status: ProposalStatus,
    pub outcome_contracts: Option<Vec<OutcomeContract>>,
}

#[contracttype]
enum Key {
    Proposal(Bytes, u32),
}

#[contract]
pub struct TansuStub;

#[contractimpl]
impl TansuStub {
    /// Tansu's real `get_proposal(project_key, proposal_id) -> Proposal`.
    /// The stub stores proposals planted via `set_*_proposal` helpers.
    pub fn get_proposal(env: &Env, project_key: Bytes, proposal_id: u32) -> Proposal {
        env.storage()
            .persistent()
            .get(&Key::Proposal(project_key, proposal_id))
            .unwrap()
    }

    /// Plant an arbitrary, fully-formed `Proposal`. Used by callers that need
    /// non-`Approved` states or unusual outcome shapes (e.g. unit tests that
    /// exercise every rejection path in `RegistryTansuManager::execute`).
    pub fn set_proposal(env: &Env, project_key: Bytes, proposal: Proposal) {
        env.storage()
            .persistent()
            .set(&Key::Proposal(project_key, proposal.id), &proposal);
    }

    /// Plant an `Approved` proposal whose single outcome is
    /// `registry.deploy(wasm_name, version, contract_name, admin, init, deployer)`.
    ///
    /// `admin` is reused as the sole constructor arg, matching the
    /// `__constructor(admin: Address)` shape of the `hello` example contract.
    /// Use [`set_proposal_outcome`] for any other shape.
    pub fn set_deploy_proposal(
        env: &Env,
        project_key: Bytes,
        proposal_id: u32,
        registry: Address,
        wasm_name: String,
        version: Option<String>,
        contract_name: String,
        admin: Address,
        deployer: Option<Address>,
    ) {
        let init: Option<Vec<Val>> = Some(vec![env, admin.clone().into_val(env)]);
        let args: Vec<Val> = vec![
            env,
            wasm_name.into_val(env),
            version.into_val(env),
            contract_name.into_val(env),
            admin.into_val(env),
            init.into_val(env),
            deployer.into_val(env),
        ];
        Self::store(
            env,
            project_key,
            proposal_id,
            registry,
            Symbol::new(env, "deploy"),
            args,
        );
    }

    /// Plant a fully custom `Approved` proposal — caller supplies the outcome
    /// `(target, fn_name, args)` directly.
    pub fn set_proposal_outcome(
        env: &Env,
        project_key: Bytes,
        proposal_id: u32,
        target: Address,
        fn_name: Symbol,
        args: Vec<Val>,
    ) {
        Self::store(env, project_key, proposal_id, target, fn_name, args);
    }

    fn store(
        env: &Env,
        project_key: Bytes,
        proposal_id: u32,
        target: Address,
        fn_name: Symbol,
        args: Vec<Val>,
    ) {
        let outcome = OutcomeContract {
            address: target,
            execute_fn: fn_name,
            args,
        };
        let proposal = Proposal {
            id: proposal_id,
            title: String::from_str(env, ""),
            proposer: env.current_contract_address(),
            ipfs: String::from_str(env, ""),
            vote_data: VoteData {
                voting_ends_at: 0,
                public_voting: true,
                token_contract: None,
                votes: Vec::<Vote>::new(env),
            },
            status: ProposalStatus::Approved,
            outcome_contracts: Some(vec![env, outcome]),
        };
        env.storage()
            .persistent()
            .set(&Key::Proposal(project_key.clone(), proposal_id), &proposal);
    }
}
