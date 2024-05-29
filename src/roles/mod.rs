use anyhow::Error;
pub mod builder;
pub mod harvester;
pub mod unknown;
pub mod upgrader;

pub trait RoleCreep {
    fn run(self) -> Result<(), Error>;
    fn should_spawn(self) -> Result<bool, Error>;
}
