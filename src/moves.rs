use serde::{Deserialize, Serialize};

use pokedex::{
    ailment::LiveAilment,
    item::ItemId,
    moves::{MoveId, PP},
    pokemon::{Experience, Level},
};

use crate::{
    pokemon::{
        battle::stat::{BattleStatType, Stage},
        PokemonIndex,
    },
    Indexed,
};

pub mod damage;
pub mod engine;
pub mod target;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BattleMove<ID> {
    /// Move (by its index), and its optional target.
    Move(usize, Option<PokemonIndex<ID>>),
    UseItem(ItemId, PokemonIndex<ID>),
    Switch(usize),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ClientMove<ID> {
    /// Id of move, PP lost from using the move, client move actions
    Move(MoveId, PP, Vec<Indexed<ID, ClientMoveAction>>),
    Switch(usize),
    UseItem(ItemId, usize),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ClientMoveAction {
    /// This contains the percent HP the pokemon was left at, how effective the attack was, and if it was a critical hit.
    /// A Pokemon faints when it's hp is set to 0.0
    SetDamage(damage::DamageResult<f32>),
    /// A Pokemon faints when it's hp is set to 0.0
    SetHP(f32),
    AddStat(BattleStatType, Stage),
    Ailment(LiveAilment),
    Miss,

    SetExp(Experience, Level),

    Error,
}

pub type Critical = bool;
/// 0 through 100
pub type Percent = u8;

impl<ID: core::fmt::Display> core::fmt::Display for BattleMove<ID> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BattleMove::Move(index, target) => write!(f, "Move #{}", index),
            BattleMove::UseItem(id, target) => write!(f, "Item {}", id),
            BattleMove::Switch(index) => write!(f, "Switch to {}", index),
        }
    }
}