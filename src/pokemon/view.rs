use pokedex::pokemon::{Level, PokemonInstance, PokemonRef};

use super::{BattlePokemon, UnknownPokemon};

pub trait PokemonView {

    fn pokemon(&self) -> PokemonRef;

    fn name(&self) -> &str;

    fn level(&self) -> Level;

    fn fainted(&self) -> bool;

    /// Check if fainted or hidden
    fn available(&self) -> bool;

}

impl PokemonView for BattlePokemon {
    fn pokemon(&self) -> PokemonRef {
        self.pokemon
    }

    fn name(&self) -> &str {
        PokemonInstance::name(self)
    }

    fn level(&self) -> Level {
        self.level
    }

    fn fainted(&self) -> bool {
        PokemonInstance::fainted(self)
    }

    fn available(&self) -> bool {
        !self.caught && PokemonInstance::available(self)
    }
}

impl PokemonView for PokemonInstance {
    fn pokemon(&self) -> PokemonRef {
        self.pokemon
    }

    fn name(&self) -> &str {
        PokemonInstance::name(self)
    }

    fn level(&self) -> Level {
        self.level
    }

    fn fainted(&self) -> bool {
        PokemonInstance::fainted(self)
    }

    fn available(&self) -> bool {
        !PokemonInstance::fainted(self)
    }
}

impl PokemonView for Option<UnknownPokemon> {
    fn pokemon(&self) -> PokemonRef {
        self.as_ref().map(|u| u.pokemon).unwrap_or_default()
    }

    fn name(&self) -> &str {
        self.as_ref().map(|u| u.name()).unwrap_or("Unknown")
    }

    fn level(&self) -> Level {
        self.as_ref().map(|u| u.level).unwrap_or_default()
    }

    fn fainted(&self) -> bool {
        self.as_ref().map(|u| u.fainted()).unwrap_or_default()
    }

    fn available(&self) -> bool {
        self.as_ref().map(|u| !u.fainted()).unwrap_or_default()
    }
}