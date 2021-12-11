use core::fmt::Debug;
use serde::{Deserialize, Serialize};

use pokedex::{moves::MoveId, pokemon::owned::SavedPokemon};

use crate::{
    moves::{BattleMove, ClientMove},
    player::ClientPlayerData,
    pokemon::{remote::RemotePokemon, ActivePosition, Indexed, PartyPosition},
};

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub enum ClientMessage<ID> {
    Move(ActivePosition, BattleMove<ID>),
    ReplaceFaint(ActivePosition, PartyPosition),
    Forfeit,
    LearnMove(PartyPosition, MoveId, Option<usize>), // pokemon index, move, move index
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ServerMessage<ID> {
    Begin(ClientPlayerData<ID>),

    Start(StartableAction<ID>),

    Ping(TimedAction),
    Fail(FailedAction),

    AddRemote(Indexed<ID, RemotePokemon>),
    Replace(Indexed<ID, usize>),

    Catch(SavedPokemon),

    PlayerEnd(EndMessage),
    GameEnd(
        /// Winner
        Option<ID>,
    ),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum StartableAction<ID> {
    Selecting,
    Turns(Vec<Indexed<ID, ClientMove<ID>>>),
}

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum TimedAction {
    Selecting,
    Replace,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum FailedAction {
    Move(ActivePosition),
    Switch(ActivePosition),
    Replace(ActivePosition),
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum EndMessage {
    Win,  // add money gained
    Lose, // add money lost
    Other,
}
