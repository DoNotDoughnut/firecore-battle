use core::{hash::Hash, ops::Deref};
use rand::Rng;
use std::{collections::HashMap, error::Error};

use pokedex::{
    item::Item,
    moves::{Move, MoveId},
    pokemon::Pokemon,
};

use crate::pokemon::{Indexed, PokemonIdentifier};

use super::{BattlePokemon, MoveEngine, MoveResult, Players};

use self::moves::*;

pub mod moves;

#[cfg(feature = "default_engine_scripting")]
pub mod scripting;

pub type Moves = HashMap<MoveId, MoveExecution>;

pub struct DefaultMoveEngine {
    pub moves: Moves,
    #[cfg(feature = "default_engine_scripting")]
    pub scripting: scripting::DefaultScriptingEngine,
}

impl DefaultMoveEngine {
    pub fn new<ID: Clone + 'static, R: Rng + Clone + 'static>() -> Self {
        Self {
            moves: Default::default(),
            #[cfg(feature = "default_engine_scripting")]
            scripting: scripting::DefaultScriptingEngine::new::<ID, R>(),
        }
    }
}

impl MoveEngine for DefaultMoveEngine {
    type Error = DefaultMoveError;

    fn execute<
        ID: Clone + Hash + Eq + 'static + core::fmt::Debug,
        R: Rng + Clone + 'static,
        P: Deref<Target = Pokemon>,
        M: Deref<Target = Move>,
        I: Deref<Target = Item>,
        PLR: Players<ID, R, P, M, I>,
    >(
        &self,
        random: &mut R,
        m: &Move,
        user: Indexed<ID, &BattlePokemon<P, M, I>>,
        targeting: Option<PokemonIdentifier<ID>>,
        players: &PLR,
    ) -> Result<Vec<Indexed<ID, MoveResult>>, Self::Error> {
        match self.moves.get(&m.id) {
            Some(usage) => {
                let targets = players.create_targets(&user.0, m, targeting, random);

                match &usage {
                    MoveExecution::Actions(actions) => {
                        let mut results = Vec::new();
                        for target_id in targets {
                            match players.get(&target_id) {
                                Some(target) => {
                                    match crate::engine::pokemon::throw_move(random, m.accuracy) {
                                        true => {
                                            results.reserve(usage.size());
                                            move_usage(
                                                &user,
                                                random,
                                                &mut results,
                                                actions,
                                                m,
                                                Indexed(target_id, target),
                                            );
                                        }
                                        false => {
                                            results.push(Indexed(user.0.clone(), MoveResult::Miss))
                                        }
                                    }
                                }
                                None => log::warn!(
                                    "Cannot get active pokemon",
                                    // target_id,
                                ),
                            }
                        }
                        Ok(results)
                    }
                    MoveExecution::Script => {
                        #[cfg(feature = "default_engine_scripting")]
                        return self.scripting.execute(random, m, user, targets, players);
                        #[cfg(not(feature = "default_engine_scripting"))]
                        return Err(DefaultMoveError::NoScriptEngine);
                    }
                    MoveExecution::None => Err(DefaultMoveError::Missing),
                }
            }
            None => Err(DefaultMoveError::Missing),
        }
    }
}

#[derive(Debug)]
pub enum DefaultMoveError {
    #[cfg(feature = "default_engine_scripting")]
    Script(Box<rhai::EvalAltResult>),
    #[cfg(not(feature = "default_engine_scripting"))]
    NoScriptEngine,
    Missing,
    NoTarget,
}

impl Error for DefaultMoveError {}

impl core::fmt::Display for DefaultMoveError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            #[cfg(feature = "default_engine_scripting")]
            Self::Script(err) => core::fmt::Display::fmt(err, f),
            other => core::fmt::Debug::fmt(other, f),
        }
    }
}
