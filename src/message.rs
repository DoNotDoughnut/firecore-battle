use core::fmt::Debug;
use serde::{Deserialize, Serialize};

use pokedex::{moves::MoveId, pokemon::owned::SavedPokemon};

use crate::{
    moves::{BattleMove, ClientMove},
    player::ValidatedPlayer,
    pokemon::{battle::UninitUnknownPokemon, ActivePosition, PartyPosition, PokemonIndex},
    Indexed,
};

#[derive(Debug, Deserialize, Serialize)]
pub enum ClientMessage<ID> {
    Move(ActivePosition, BattleMove<ID>),
    ReplaceFaint(ActivePosition, PartyPosition),
    FinishedTurnQueue,
    Forfeit,
    LearnMove(PartyPosition, MoveId, u8), // pokemon index, move, move index (0 - 3)
}

#[derive(Debug, Deserialize, Serialize)]
pub enum ServerMessage<ID, const AS: usize> {
    Begin(ValidatedPlayer<ID, AS>),
    StartSelecting,
    Catch(SavedPokemon),
    TurnQueue(Vec<Indexed<ID, ClientMove<ID>>>),
    ConfirmFaintReplace(ActivePosition, bool),
    FaintReplace(PokemonIndex<ID>, usize),
    AddUnknown(PokemonIndex<ID>, UninitUnknownPokemon),
    End(EndState),
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum EndState {
    Win,  // add money gained
    Lose, // add money lost
    Other,
}
