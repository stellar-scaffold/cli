#![no_std]

use soroban_sdk::{self, contract, contractimpl, Address, Bytes, BytesN, Env, String, Symbol, Val};
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
    /// `OutcomeContract`. The outcome's `address` must be this contract — i.e.
    /// the proposal points at one of the no-op proxies on this manager (e.g.
    /// [`publish_hash`]). Tansu's `execute` auto-invokes outcomes inline via
    /// `env.try_invoke_contract`, so an outcome targeting the registry
    /// directly would fail at the registry's `manager.require_auth()` (Tansu
    /// isn't in that chain — this manager is) and revert the whole Tansu tx.
    /// Routing through a no-op proxy here lets Tansu's auto-invocation succeed,
    /// the proposal flip to `Approved`, and then an external caller invokes
    /// `manager.execute(proposal_id)` to do the real registry forward with
    /// this contract's auth.
    ///
    /// The forward itself is untyped — `oc.execute_fn` and `oc.args` are
    /// passed through to the registry as-is. Any registry method we want to
    /// gate behind a proposal just needs a matching no-op proxy added to this
    /// contract; `execute` is not hardcoded to a specific method.
    ///
    /// `execute` is rejected as a forward target to prevent recursive
    /// re-entry through a maliciously crafted outcome.
    ///
    /// Replay-protected: a successful `execute` marks the proposal as
    /// executed; later calls with the same `proposal_id` return
    /// `AlreadyExecuted`.
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
        if oc.address != env.current_contract_address() {
            return Err(Error::OutcomeTargetMismatch);
        }
        if oc.execute_fn == Symbol::new(env, "execute") {
            // Don't let a crafted outcome recurse back into us.
            return Err(Error::OutcomeTargetMismatch);
        }

        Storage::set_executed(env, &proposal_id, &true);
        let result: Val = env.invoke_contract(&registry, &oc.execute_fn, oc.args);
        Ok(result)
    }

    // -----------------------------------------------------------------------
    // No-op proxy methods.
    //
    // Each one mirrors the signature of a registry method we want to gate
    // behind a Tansu proposal. The proposal's outcome targets one of these by
    // name + args; Tansu's auto-invocation lands here (does nothing); then
    // `execute(proposal_id)` re-reads the same outcome and forwards
    // `execute_fn + args` to the registry with this contract's auth chain.
    //
    // Adding support for another gated registry method = add another no-op
    // proxy below with the matching signature. `execute` itself is unchanged.
    // -----------------------------------------------------------------------

    /// No-op proxy for `Registry::publish_hash`.
    pub fn publish_hash(
        _env: &Env,
        _wasm_name: String,
        _author: Address,
        _wasm_hash: BytesN<32>,
        _version: String,
    ) {
    }

    /// No-op proxy used by unit tests that exercise the forward path against
    /// the `registry-stub` fixture's `manager_only(value: u32)` method.
    pub fn manager_only(_env: &Env, _value: u32) {}
}

#[cfg(test)]
mod test;
