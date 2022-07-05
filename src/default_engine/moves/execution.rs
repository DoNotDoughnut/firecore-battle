use core::ops::Deref;

use rand::Rng;

use serde::{Deserialize, Serialize};

use pokedex::{
    ailment::{Ailment, AilmentLength},
    item::Item,
    moves::Move,
    pokemon::Pokemon,
};

use crate::{
    engine::BattlePokemon,
    moves::{damage::DamageKind, engine::MoveResult, MoveCancel, Percent},
    pokemon::{
        stat::{BattleStatType, Stage},
        Indexed,
    },
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum MoveExecution {
    /// Load a vector of actions
    Actions(Vec<MoveUse>),
    /// Use a script defined in the instance of the object that uses this
    Script,
    /// Placeholder to show that object does not have a defined use yet.
    None,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum MoveUse {
    Damage(DamageKind),
    Ailment(Option<(Ailment, AilmentLength)>, Percent),
    Drain(DamageKind, i8),
    Stat(BattleStatType, Stage),
    Flinch,
    Chance(Vec<Self>, Percent),
}

impl MoveExecution {
    pub fn size(&self) -> usize {
        match self {
            Self::Actions(actions) => actions.iter().map(MoveUse::size).sum(),
            Self::Script | Self::None => 1,
        }
    }
}

impl MoveUse {
    pub fn size(&self) -> usize {
        match self {
            Self::Chance(uses, ..) => uses.iter().map(Self::size).sum(),
            Self::Drain(..) => 2,
            _ => 1,
        }
    }
}

pub fn move_usage<
    'd,
    ID: Clone,
    R: Rng,
    P: Deref<Target = Pokemon> + Clone,
    M: Deref<Target = Move> + Clone,
    I: Deref<Target = Item> + Clone,
>(
    user: &Indexed<ID, &BattlePokemon<P, M, I>>,
    random: &mut R,
    results: &mut Vec<Indexed<ID, MoveResult>>,
    actions: &[MoveUse],
    m: &Move,
    Indexed(target_id, target): Indexed<ID, &BattlePokemon<P, M, I>>,
) {
    for action in actions {
        match action {
            MoveUse::Damage(kind) => {
                results.push(Indexed(
                    target_id.clone(),
                    MoveResult::Damage(user.1.damage_kind(
                        random,
                        target,
                        *kind,
                        m.category,
                        m.pokemon_type,
                        m.crit_rate,
                    )),
                ));
            }
            MoveUse::Ailment(effect, chance) => {
                if random.gen_bool(*chance as f64 / 100.0) {
                    match effect {
                        Some((ailment, length)) => {
                            if target.ailment.is_none() {
                                results.push(Indexed(
                                    target_id.clone(),
                                    MoveResult::Ailment(Some(length.init(*ailment, random))),
                                ));
                            }
                        }
                        None => {
                            if target.ailment.is_some() {
                                results.push(Indexed(
                                    target_id.clone(),
                                    MoveResult::Ailment(None),
                                ))
                            }
                        },
                    }
                }
            }
            MoveUse::Drain(kind, percent) => {
                let result = user.1.damage_kind(
                    random,
                    target,
                    *kind,
                    m.category,
                    m.pokemon_type,
                    m.crit_rate,
                );

                let healing = (result.damage as f32 * *percent as f32 / 100.0) as i16;

                results.push(Indexed(target_id.clone(), MoveResult::Damage(result)));
                results.push(Indexed(user.0.clone(), MoveResult::Heal(healing)))
            }
            MoveUse::Stat(stat, stage) => {
                if target.stages.can_change(*stat, *stage) {
                    results.push(Indexed(target_id.clone(), MoveResult::Stat(*stat, *stage)));
                }
            }
            // MoveUseType::Linger(..) => {
            // 	results.insert(target.instance, Some(MoveAction::Todo));
            // }
            MoveUse::Flinch => results.push(Indexed(
                target_id.clone(),
                MoveResult::Cancel(MoveCancel::Flinch),
            )),
            MoveUse::Chance(actions, chance) => {
                if random.gen_range(0..=100) < *chance {
                    move_usage(
                        user,
                        random,
                        results,
                        actions,
                        m,
                        Indexed(target_id.clone(), target),
                    );
                }
            }
        }
    }
}
