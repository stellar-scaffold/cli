use super::diagnosis::Check;

pub mod network;
pub mod project;
pub mod toolchain;

/// Declaration order _is_ output order. Cheap, local checks first so a broken
/// toolchain is reported before slow network probe runs.
pub fn registry() -> Vec<Box<dyn Check>> {
    vec![
        Box::new(toolchain::Rustc),
        Box::new(toolchain::WasmTarget),
        Box::new(toolchain::StellarCli),
        Box::new(toolchain::NodeToolchain),
        Box::new(project::ScaffoldYml),
        Box::new(project::EngineConstraint),
        Box::new(project::DotEnv),
        Box::new(project::EnvironmentsToml),
        Box::new(project::Extensions),
        Box::new(network::DockerDaemon),
        Box::new(network::Localnet),
    ]
}
