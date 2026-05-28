#![no_std]
// Tansu-stub-generated client + types include multi-arg fns (e.g.
// `set_deploy_proposal`), which trip `too_many_arguments`. Allow on the lib
// since the lint fires inside the macro expansion of `import_contract_client!`.
#![allow(clippy::too_many_arguments)]

use soroban_sdk::{
    self,
    auth::{ContractContext, InvokerContractAuthEntry, SubContractInvocation},
    contract, contractimpl, vec, Address, Bytes, Env, IntoVal, Symbol, Val, Vec,
};
use soroban_sdk_tools::{contractstorage, InstanceItem};

#[soroban_sdk_tools::scerr]
pub enum Error {
    /// Proposal has no outcomes attached.
    NoOutcomeContracts,
    /// Proposal has more than one outcome — this manager authorizes exactly
    /// one sub-call per proposal.
    MultipleOutcomes,
}

// Proposal/status types are derived from the tansu-stub contract's wasm spec.
// At runtime this manager points at *real* Tansu; the stub matches Tansu's
// wire format so the generated `get_proposal` client decodes a live proposal
// correctly.
stellar_registry::import_contract_client!(tansu_stub);

#[contractstorage(auto_shorten = true)]
pub struct Storage {
    /// Tansu DAO contract whose proposals this manager drives.
    tansu: InstanceItem<Address>,
    /// Tansu workspace key this manager represents. All Tansu lookups are
    /// keyed by this — a wrong-project caller can't piggyback.
    project_key: InstanceItem<Bytes>,
    /// Registry this manager is the manager of. Recorded for inspection;
    /// `trigger` doesn't read it directly because it uses whatever outcome
    /// the (project_key-gated) proposal carries.
    registry: InstanceItem<Address>,
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

    /// Drive a Tansu proposal through to outcome execution in one transaction.
    ///
    /// Flow:
    ///
    /// 1. Read the proposal from this manager's configured Tansu under this
    ///    manager's configured `project_key`. Wrong-project callers can't
    ///    construct a working invocation — `get_proposal` is keyed by
    ///    `(project_key, proposal_id)` Tansu-side, so any mismatched proposal
    ///    decodes to whatever lives at that key in *our* DAO or panics.
    /// 2. Take the single approved-branch outcome (`outcome_contracts[0]`):
    ///    its `address`, `execute_fn`, and `args`.
    /// 3. Pre-authorize **this contract's auth** for exactly that one
    ///    sub-call via `env.authorize_as_current_contract(...)`. Nothing
    ///    else gets authorized. The auth entry is scoped to one specific
    ///    `(contract, fn, args)` triple.
    /// 4. Call `Tansu.execute(maintainer, project_key, proposal_id, _, _)`.
    ///    Tansu tallies the votes, sets the proposal to its terminal status,
    ///    and (on `Approved`) auto-invokes the outcome. When that outcome
    ///    reaches `manager.require_auth()`, the host matches it against the
    ///    pre-authorization from step 3 and lets the call run.
    ///
    /// For this to work the manager must be the Tansu project's maintainer
    /// (set up at deploy time via `Tansu::register(..., maintainers=[manager])`
    /// or `update_config`). That way the manager is the direct caller of
    /// `Tansu::execute`, so Tansu's internal `maintainer.require_auth()` is
    /// satisfied by contract-implicit auth — no auth entry needed for the
    /// maintainer requirement, no non-root recording issue.
    ///
    /// Tansu's own `if proposal.status != Active` guard inside `execute`
    /// prevents the same proposal being triggered twice — no separate
    /// replay guard needed here.
    pub fn trigger(env: &Env, proposal_id: u32) -> Result<(), Error> {
        let tansu = Storage::get_tansu(env).unwrap();
        let project_key = Storage::get_project_key(env).unwrap();

        let proposal =
            tansu_stub::Client::new(env, &tansu).get_proposal(&project_key, &proposal_id);
        let outcomes = proposal
            .outcome_contracts
            .ok_or(Error::NoOutcomeContracts)?;
        if outcomes.len() != 1 {
            return Err(Error::MultipleOutcomes);
        }
        let oc = outcomes.get(0).unwrap();

        env.authorize_as_current_contract(vec![
            env,
            InvokerContractAuthEntry::Contract(SubContractInvocation {
                context: ContractContext {
                    contract: oc.address.clone(),
                    fn_name: oc.execute_fn.clone(),
                    args: oc.args.clone(),
                },
                sub_invocations: Vec::new(env),
            }),
        ]);

        // Tansu.execute(maintainer, project_key, proposal_id, tallies, seeds).
        // maintainer = self — must match the project's `maintainers` list in
        // Tansu (configured at registration / update_config time).
        let _: Val = env.invoke_contract(
            &tansu,
            &Symbol::new(env, "execute"),
            vec![
                env,
                env.current_contract_address().into_val(env),
                project_key.into_val(env),
                proposal_id.into_val(env),
                None::<Vec<u128>>.into_val(env),
                None::<Vec<u128>>.into_val(env),
            ],
        );
        Ok(())
    }
}

#[cfg(test)]
mod test;
