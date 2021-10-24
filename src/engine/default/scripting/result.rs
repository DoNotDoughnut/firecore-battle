use rhai::INT;

use crate::{engine::MoveResult, pokemon::Indexed};

use super::{damage::ScriptDamage, pokemon::ScriptPokemon};

#[derive(Clone, Copy)]
pub struct ScriptMoveResult<ID>(pub Indexed<ID, MoveResult>);

impl<ID> ScriptMoveResult<ID> {
    pub fn new(pokemon: ScriptPokemon<ID>, result: MoveResult) -> Self {
        Self(Indexed(pokemon.into(), result))
    }

    pub fn damage(
        pokemon: ScriptPokemon<ID>,
        damage: ScriptDamage,
    ) -> Self {
        Self::new(pokemon, MoveResult::Damage(damage.into()))
    }

    pub fn heal(
        pokemon: ScriptPokemon<ID>,
        heal: INT,
    ) -> Self {
        Self::new(pokemon, MoveResult::Heal(heal as _))
    }

    // pub const fn Status(effect: StatusEffect) -> MoveResult { MoveResult::Status(effect) }

    pub fn miss(pokemon: ScriptPokemon<ID>) -> Self {
        Self::new(pokemon, MoveResult::Miss)
    }

}
