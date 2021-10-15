use core::{cmp::Reverse, fmt::Display, hash::Hash};
use log::{info, warn};
use rand::Rng;

use pokedex::{
    item::{usage::ItemUsageKind, Item},
    moves::{owned::OwnedMove, Move, Priority},
    pokemon::{stat::{BaseStat, StatType}, Health, Pokemon},
    Dex, Uninitializable,
};

use crate::{
    data::*,
    message::{ClientMessage, EndState, ServerMessage},
    moves::{damage::DamageResult, BattleMove, ClientMove, ClientMoveAction},
    party::PartyIndex,
    player::{LocalPlayer, ValidatedPlayer},
    pokemon::PokemonIdentifier,
    BattleEndpoint, Indexed,
};

mod collections;
mod party;
mod pokemon;

pub mod engine;
pub mod player;

use collections::BattleMap;
use engine::{MoveEngine, MoveResult};
use player::BattlePlayer;
use party::BattleParty;
use pokemon::BattlePokemon;
use std::collections::BTreeMap;

pub mod prelude {

    #[cfg(feature = "default_engine")]
    pub use super::engine::default::*;

    pub use super::engine::{MoveEngine, MoveResult};
    pub use super::Battle;

}

pub struct Battle<
    'd,
    ID: Display + Clone + Ord + Hash + 'static,
    E: BattleEndpoint<ID, AS>,
    const AS: usize,
> {
    state: BattleState,
    data: BattleData,
    players: BattleMap<ID, BattlePlayer<'d, ID, E, AS>>, // can change to dex implementation, and impl Identifiable for BattlePlayer
}

#[derive(Debug)]
enum BattleState {
    Start,
    StartSelecting,
    WaitSelecting,
    StartMoving,
    WaitMoving,
    End,
}

impl Default for BattleState {
    fn default() -> Self {
        Self::Start
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MovePriority<ID: Ord> {
    First(ID, usize),
    Second(Reverse<Priority>, Reverse<BaseStat>, Option<u16>),
}

impl<
        'd,
        ID: Display + Clone + Ord + Hash + 'static,
        E: BattleEndpoint<ID, AS>,
        const AS: usize,
    > Battle<'d, ID, E, AS>
{
    pub fn new<R: Rng>(
        data: BattleData,
        random: &mut R,
        pokedex: &'d impl Dex<Pokemon>,
        movedex: &'d impl Dex<Move>,
        itemdex: &'d impl Dex<Item>,
        players: impl IntoIterator<Item = LocalPlayer<ID, E, AS>>,
    ) -> Self {
        Self {
            state: BattleState::default(),
            data,
            players: players
                .into_iter()
                .map(|p| {
                    let p = p.init(random, pokedex, movedex, itemdex);
                    (p.id().clone(), p)
                })
                .collect(),
        }
    }

    pub fn begin(&mut self) {
        for mut player in self.players.values_mut() {
            player.party.reveal_active();
        }

        for mut player in self.players.values_mut() {
            let v = ValidatedPlayer::new(self.data.clone(), &player, self.players.values());
            player.send(ServerMessage::Begin(v));
        }

        self.state = BattleState::StartSelecting;
    }

    pub fn end(&mut self, winner: Option<ID>) {
        self.state = BattleState::End;
        for (id, mut player) in self.players.iter_mut() {
            match Some(id) == winner.as_ref() {
                true => player.send(ServerMessage::End(EndState::Win)),
                false => player.send(ServerMessage::End(EndState::Lose)),
            }
        }
    }

    pub fn update<R: Rng + Clone + 'static, ENG: MoveEngine>(
        &mut self,
        random: &mut R,
        engine: &mut ENG,
        itemdex: &'d impl Dex<Item>,
    ) {
        for (waiting, mut player) in self.players.values_waiting_mut() {
            while let Some(message) = player.receive() {
                match message {
                    ClientMessage::Move(active, bmove) => {
                        match player
                            .party
                            .active
                            .get_mut(active)
                            .map(Option::as_mut)
                            .flatten()
                        {
                            Some(pokemon) => {
                                pokemon.queued_move = Some(bmove);
                                // party.client.confirm_move(active);
                            }
                            None => warn!(
                                "Party {} could not add move #{} to pokemon #{}",
                                player.party.name(),
                                bmove,
                                active
                            ),
                        }
                    }
                    ClientMessage::ReplaceFaint(active, index) => {
                        match player.party.active_contains(index) {
                            false => match player.party.pokemon.get(index) {
                                Some(pokemon) => match pokemon.fainted() {
                                    false => {
                                        player.party.active[active] = Some(index.into());
                                        let unknown = player.party.know(index);
                                        for mut other in self.players.values_mut() {
                                            if let Some(pokemon) = unknown.as_ref() {
                                                other.send(ServerMessage::AddRemote(
                                                    PokemonIdentifier(
                                                        player.party.id().clone(),
                                                        index,
                                                    ),
                                                    pokemon.clone(),
                                                ));
                                            }
                                            other.send(ServerMessage::FaintReplace(
                                                PokemonIdentifier(
                                                    player.party.id().clone(),
                                                    active,
                                                ),
                                                index,
                                            ));
                                        }

                                        player
                                            .send(ServerMessage::ConfirmFaintReplace(active, true));
                                    }
                                    true => player
                                        .send(ServerMessage::ConfirmFaintReplace(active, false)),
                                },
                                None => {
                                    player.send(ServerMessage::ConfirmFaintReplace(active, false))
                                }
                            },
                            true => player.send(ServerMessage::ConfirmFaintReplace(active, false)),
                        }
                    }
                    ClientMessage::Forfeit => {
                        // self.remove(player.party.id);
                        // Self::set_winner(Some(other.party.id), player, other),
                    }
                    ClientMessage::LearnMove(pokemon, move_id, index) => {
                        if let Some(pokemon) = player.party.pokemon.get_mut(pokemon) {
                            if pokemon.learnable_moves.remove(&move_id) {
                                pokemon.moves.add(Some(index as _), &move_id);
                            }
                        }
                    }
                    ClientMessage::FinishedTurnQueue => waiting.set(true),
                }
            }
        }

        match self.state {
            BattleState::Start => self.begin(),
            BattleState::StartSelecting => {
                for (waiting, mut player) in self.players.values_waiting_mut() {
                    waiting.set(false);
                    player.send(ServerMessage::StartSelecting);
                }
                self.state = BattleState::WaitSelecting;
            }
            BattleState::WaitSelecting => {
                if self.players.values().all(|p| p.party.ready_to_move()) {
                    self.state = BattleState::StartMoving;
                }
            }
            BattleState::StartMoving => {
                let queue = move_queue(&mut self.players, random);

                let player_queue = self.client_queue(random, engine, itemdex, queue);

                // end queue calculations

                for mut player in self.players.values_mut() {
                    player.send(ServerMessage::TurnQueue(player_queue.clone()));
                }

                self.state = BattleState::WaitMoving;
            }
            BattleState::WaitMoving => {
                if self.players.all_waiting() {
                    for (id, player) in self.players.iter() {
                        if player.party.all_fainted() {
                            self.remove_player(id, EndState::Lose);
                        }
                    }

                    if self.players.active() <= 1 {
                        let winner = self.players.keys().next().cloned();
                        self.end(winner);
                        return;
                    } else if self.players.values().all(|p| !p.party.needs_replace()) {
                        self.state = BattleState::StartSelecting;
                    }
                }
            }
            BattleState::End => (),
        }
    }

    pub fn remove_player(&self, id: &ID, kind: EndState) {
        if let Some(mut player) = self.players.deactivate(id) {
            player.send(ServerMessage::End(kind));
        }
    }

    pub fn finished(&self) -> bool {
        matches!(self.state, BattleState::End)
    }

    pub fn data(&self) -> &BattleData {
        &self.data
    }

    pub fn winner(&self) -> Option<ID> {
        match &self.state {
            BattleState::End => self.players.values().next().map(|p| p.id().clone()),
            _ => None,
        }
    }

    pub fn client_queue<R: Rng + Clone + 'static, ENG: MoveEngine>(
        &mut self,
        random: &mut R,
        engine: &mut ENG,
        itemdex: &'d impl Dex<Item>,
        queue: Vec<Indexed<ID, BattleMove<ID>>>,
    ) -> Vec<Indexed<ID, ClientMove<ID>>> {
        let mut player_queue = Vec::with_capacity(queue.len());

        for Indexed(user_id, bmove) in queue {
            match bmove {
                BattleMove::Move(move_index, targets) => {
                    let turn = match self.players.get(user_id.team()) {
                        Some(user) => match user.party.active(user_id.index()) {
                            Some(pokemon) => {
                                // match Some(pokemon.moves.get(0).unwrap().0)
                                match pokemon
                                    .moves
                                    .get(move_index)
                                    .map(OwnedMove::try_use)
                                    .flatten() 
                                    {
                                        Some(used_move) => engine.execute(random, used_move, Indexed(user_id.clone(), pokemon), targets, &self.players).map_err(|err| {
                                            warn!("Cannot execute move {} for user {}'s pokemon {} with error {}", used_move.name, user.name(), pokemon.name(), err);
                                        }).ok().map(|turn| (used_move.id, turn)),
                                        None => {
                                            log::warn!("Cannot use move #{} for user {}'s {}", move_index, user.name(), pokemon.name());
                                            None
                                        },
                                    }
                            }
                            None => None,
                        },
                        None => {
                            log::error!("Cannot get user {}!", user_id.team());
                            None
                        }
                    };

                    if let Some((used_move, output)) = turn {
                        let mut actions = Vec::with_capacity(output.len());

                        for Indexed(target_id, action) in output {
                            match self.players.get_mut(target_id.team()) {
                                Some(mut player) => {
                                    match player.party.active_mut(target_id.index()) {
                                        Some(target) => {
                                            /// calculates hp and adds it to actions
                                            fn on_damage<'d, ID>(
                                                location: PokemonIdentifier<ID>,
                                                pokemon: &mut BattlePokemon<'d>,
                                                actions: &mut Vec<Indexed<ID, ClientMoveAction>>,
                                                result: DamageResult<Health>,
                                            ) {
                                                pokemon.hp =
                                                    pokemon.hp.saturating_sub(result.damage);
                                                actions.push(Indexed(
                                                    location,
                                                    ClientMoveAction::SetDamage(DamageResult {
                                                        damage: pokemon.percent_hp(),
                                                        effective: result.effective,
                                                        crit: result.crit,
                                                    }),
                                                ));
                                            }

                                            let t_id = target_id.clone();

                                            match action {
                                                MoveResult::Damage(result) => on_damage(
                                                    target_id,
                                                    target,
                                                    &mut actions,
                                                    result,
                                                ),
                                                MoveResult::Ailment(ailment) => {
                                                    target.ailment = Some(ailment);
                                                    actions.push(Indexed(
                                                        target_id,
                                                        ClientMoveAction::Ailment(ailment),
                                                    ));
                                                }
                                                MoveResult::Heal(health) => {
                                                    let hp = health.abs() as u16;
                                                    target.hp = match health.is_positive() {
                                                        true => target.hp + hp.min(target.max_hp()),
                                                        false => target.hp.saturating_sub(hp),
                                                    };
                                                    actions.push(Indexed(
                                                        target_id,
                                                        ClientMoveAction::SetHP(
                                                            target.percent_hp(),
                                                        ),
                                                    ));
                                                }
                                                MoveResult::Stat(stat, stage) => {
                                                    target.stages.change_stage(stat, stage);
                                                    actions.push(Indexed(
                                                        target_id,
                                                        ClientMoveAction::AddStat(stat, stage),
                                                    ));
                                                }
                                                MoveResult::Flinch => target.flinch = true,
                                                MoveResult::Miss => {
                                                    actions.push(Indexed(
                                                        target_id,
                                                        ClientMoveAction::Miss,
                                                    ));
                                                    continue;
                                                }
                                                MoveResult::Error => actions.push(Indexed(
                                                    target_id,
                                                    ClientMoveAction::Error,
                                                )),
                                            }

                                            if target.fainted() {

                                                let experience =
                                                    target.battle_exp_from(&self.data.type_);

                                                player.party.remove_active(t_id.index());

                                                if player.id() != t_id.team() {
                                                    let mut user = self
                                                        .players
                                                        .get_mut(user_id.team())
                                                        .unwrap();

                                                    if user.settings.gains_exp {
                                                        let user = user
                                                            .party
                                                            .active_mut(user_id.index())
                                                            .unwrap();

                                                        user.learnable_moves.extend(
                                                            user.instance.add_exp(experience),
                                                        );

                                                        actions.push(Indexed(
                                                            user_id.clone(),
                                                            ClientMoveAction::SetExp(
                                                                experience, user.level,
                                                            ),
                                                        ));
                                                    }
                                                }
                                            }
                                        }
                                        None => {}
                                    }
                                }
                                None => {}
                            }
                        }

                        let mut user = self.players.get_mut(user_id.team()).unwrap();

                        if let Some(pokemon) = user.party.active_mut(user_id.index()) {
                            // decrement PP
                            pokemon.moves.get_mut(move_index).unwrap().decrement();

                            if pokemon.fainted() {
                                user.party.remove_active(user_id.index());
                            }
                        }

                        const TEMP_PP: u8 = 1;

                        player_queue.push(Indexed(
                            user_id,
                            ClientMove::Move(used_move, TEMP_PP, actions),
                        ));
                    }
                }
                BattleMove::UseItem(id, target) => match itemdex.try_get(&id) {
                    Some(item) => {
                        let mut user = match self.players.get_mut(user_id.team()) {
                            Some(p) => p,
                            None => continue,
                        };

                        if match &item.usage.kind {
                            ItemUsageKind::Script | ItemUsageKind::Actions(..) => {
                                match user.party.active_mut(target.index()) {
                                    Some(pokemon) => pokemon.try_use_item(&item),
                                    None => false,
                                }
                            }
                            ItemUsageKind::Pokeball => match self.data.type_ {
                                BattleType::Wild => {
                                    if let Some(mut other) = self.players.get_mut(target.team()) {
                                        if let Some(active) = other
                                            .party
                                            .active
                                            .get_mut(target.index())
                                            .map(Option::take)
                                            .flatten()
                                        {
                                            if let Some(pokemon) = other.party.take(active.index())
                                            {
                                                user.send(ServerMessage::Catch(
                                                    pokemon.instance.uninit(),
                                                ));

                                                true
                                            } else {
                                                false
                                            }
                                        } else {
                                            false
                                        }
                                    } else {
                                        false
                                    }
                                }
                                _ => {
                                    info!("Cannot use pokeballs in trainer battles!");
                                    false
                                }
                            },
                            ItemUsageKind::None => true,
                        } {
                            player_queue.push(Indexed(
                                user_id,
                                ClientMove::UseItem(item.id, target.index()),
                            ));
                        }
                    }
                    None => warn!("Could not get item with id {}", id),
                },
                BattleMove::Switch(new) => match self.players.get_mut(user_id.team()) {
                    Some(mut user) => {
                        user.party.replace(user_id.index(), Some(new));

                        if let Some(unknown) = user
                            .party
                            .index(user_id.index())
                            .map(|index| user.party.know(index))
                            .flatten()
                        {
                            for mut other in self.players.values_mut() {
                                other.send(ServerMessage::AddRemote(
                                    PokemonIdentifier(user.party.id().clone(), new),
                                    unknown.clone(),
                                ))
                            }
                        }
                        player_queue.push(Indexed(user_id, ClientMove::Switch(new)));
                    }
                    None => todo!(),
                },
            }
        }
        player_queue
    }
}

pub fn move_queue<ID: Clone + Ord + Hash, E: BattleEndpoint<ID, AS>, R: Rng, const AS: usize>(
    players: &mut BattleMap<ID, BattlePlayer<ID, E, AS>>,
    random: &mut R,
) -> Vec<Indexed<ID, BattleMove<ID>>> {
    let mut queue = BTreeMap::new();

    for mut player in players.values_mut() {
        queue_player(&mut queue, &mut player.party, random)
    }

    queue.into_values().collect()
}

fn queue_player<ID: Clone + Ord, R: Rng, const AS: usize>(
    queue: &mut BTreeMap<MovePriority<ID>, Indexed<ID, BattleMove<ID>>>,
    party: &mut BattleParty<ID, AS>,
    random: &mut R,
) {
    for index in 0..AS {
        if let Some(pokemon) = party.active.get_mut(index).map(Option::as_mut).flatten() {
            if let Some(action) = pokemon.queued_move.take() {
                if let Some(instance) = party.active(index) {
                    let pokemon = PokemonIdentifier(party.id().clone(), index);

                    let mut priority = match action {
                        BattleMove::Move(index, ..) => MovePriority::Second(
                            Reverse(
                                instance
                                    .moves
                                    .get(index)
                                    .map(|i| i.0.priority)
                                    .unwrap_or_default(),
                            ),
                            Reverse(instance.stat(StatType::Speed)),
                            None,
                        ),
                        _ => MovePriority::First(party.id().clone(), index),
                    };

                    fn tie_break<ID: Ord, R: Rng>(
                        queue: &mut BTreeMap<MovePriority<ID>, Indexed<ID, BattleMove<ID>>>,
                        random: &mut R,
                        priority: &mut MovePriority<ID>,
                    ) {
                        if let MovePriority::Second(.., shift) = priority {
                            *shift = Some(random.gen());
                        }
                        if queue.contains_key(priority) {
                            tie_break(queue, random, priority);
                        }
                    }

                    if queue.contains_key(&priority) {
                        tie_break(queue, random, &mut priority);
                    }

                    queue.insert(priority, Indexed(pokemon, action));
                }
            }
        }
    }
}
