#![no_std]

use soroban_sdk::{self, contract, contractimpl, Address, Bytes, Env, Val};
use soroban_sdk_tools::{contractstorage, InstanceItem, PersistentMap};

#[soroban_sdk_tools::scerr]
pub enum Error {
    /// Proposal exists but is not in the `Approved` state.
    NotApproved,
    /// Proposal has no outcome contracts attached.
    NoOutcomeContracts,
    /// Proposal has more than one outcome contract.
    MultipleOutcomes,
    /// Proposal's outcome targets an address other than the configured registry.
    OutcomeTargetMismatch,
    /// Proposal has already been executed by this manager.
    AlreadyExecuted,
}

// Tansu proposal types + client come from the `tansu-stub` contract's wasm
// (built by `stellar scaffold build` ahead of this crate via the Cargo edge in
// `[dependencies]`). The stub is the single source of truth for these types,
// hand-mirrored from upstream Tansu — see `contracts/test/tansu-stub/src/lib.rs`.
// At runtime the manager points its `tansu` Address at *real* Tansu; the
// wire-level encoding matches because the stub mirrors Tansu's spec.
stellar_registry::import_contract_client!(tansu_stub);

#[contractstorage(auto_shorten = true)]
pub struct Storage {
    /// Tansu DAO contract this manager queries proposals from.
    tansu: InstanceItem<Address>,
    /// Tansu workspace key this manager represents.
    project_key: InstanceItem<Bytes>,
    /// Registry contract this manager forwards approved outcomes to.
    registry: InstanceItem<Address>,
    /// Proposal IDs that have already been executed (replay guard).
    executed: PersistentMap<u32, bool>,
}

#[contract]
pub struct RegistryTansuManager;

#[contractimpl]
impl RegistryTansuManager {
    pub fn __constructor(env: &Env, tansu: &Address, project_key: &Bytes, registry: &Address) {
        Storage::set_tansu(env, tansu);
        Storage::set_project_key(env, project_key);
        Storage::set_registry(env, registry);
    }

    pub fn tansu(env: &Env) -> Address {
        Storage::get_tansu(env).unwrap()
    }

    pub fn project_key(env: &Env) -> Bytes {
        Storage::get_project_key(env).unwrap()
    }

    pub fn registry(env: &Env) -> Address {
        Storage::get_registry(env).unwrap()
    }

    /// Execute a passed Tansu proposal by forwarding its outcome to the registry.
    ///
    /// The proposal must be in `Approved` state and carry exactly one
    /// `OutcomeContract` whose `address` matches the configured registry. The
    /// outcome's `execute_fn` + `args` are forwarded via XCC — the registry's
    /// `manager.require_auth()` is satisfied automatically because this
    /// contract is the direct caller (Soroban contract-auth chains for
    /// outgoing invocations; no `authorize_as_current_contract` is needed).
    ///
    /// Replay-protected: a successful `execute` marks the proposal as
    /// executed; later calls with the same `proposal_id` return
    /// `AlreadyExecuted`.
    ///
    /// Trust: we look up the proposal in Tansu using the stored `project_key`,
    /// so a wrong-project proposal cannot resolve. We do not re-verify
    /// `project_key` against any field of the returned proposal — Tansu's
    /// storage layout makes that lookup the only path.
    ///
    /// The forward call to the registry stays as untyped `env.invoke_contract`
    /// rather than a typed client: an approved proposal can target *any*
    /// registry method (whatever `execute_fn` + `args` the DAO passed), and a
    /// typed client can't express that arbitrary forward.
    pub fn execute(env: &Env, proposal_id: u32) -> Result<Val, Error> {
        if Storage::has_executed(env, &proposal_id) {
            return Err(Error::AlreadyExecuted);
        }
        let tansu = Storage::get_tansu(env).unwrap();
        let project_key = Storage::get_project_key(env).unwrap();
        let registry = Storage::get_registry(env).unwrap();

        let proposal =
            tansu_stub::Client::new(env, &tansu).get_proposal(&project_key, &proposal_id);

        if !matches!(proposal.status, tansu_stub::ProposalStatus::Approved) {
            return Err(Error::NotApproved);
        }
        let outcomes = proposal
            .outcome_contracts
            .ok_or(Error::NoOutcomeContracts)?;
        if outcomes.len() != 1 {
            return Err(Error::MultipleOutcomes);
        }
        let oc = outcomes.get(0).unwrap();
        if oc.address != registry {
            return Err(Error::OutcomeTargetMismatch);
        }

        let result: Val = env.invoke_contract(&registry, &oc.execute_fn, oc.args);
        Storage::set_executed(env, &proposal_id, &true);
        Ok(result)
    }
}

#[cfg(test)]
mod test;
