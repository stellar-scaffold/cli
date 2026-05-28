#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env, String};

#[contract]
pub struct Hello;

#[contractimpl]
impl Hello {
    pub fn __constructor(env: &Env, admin: &Address) {
        env.storage().instance().set(&"admin", admin);
    }

    pub fn admin(env: &Env) -> Address {
        env.storage().instance().get(&"admin").unwrap()
    }

    pub fn hello(_env: &Env, to: String) -> String {
        to
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn hello_echoes() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let id = env.register(Hello, (admin.clone(),));
        let c = HelloClient::new(&env, &id);
        assert_eq!(
            c.hello(&String::from_str(&env, "world")),
            String::from_str(&env, "world")
        );
        assert_eq!(c.admin(), admin);
    }
}
