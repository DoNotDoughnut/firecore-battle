use core::{cell::Ref, ops::Deref};
use rand::Rng;

use pokedex::{
    item::{bag::SavedBag, Item},
    moves::Move,
    pokemon::{owned::SavedPokemon, party::Party, Pokemon},
    Dex,
};

use crate::{
    data::BattleData,
    endpoint::{BattleEndpoint, ReceiveError},
    message::{ClientMessage, ServerMessage},
    party::{ActivePokemon, PlayerParty},
    player::{ClientPlayerData, Player, PlayerSettings},
};

use super::pokemon::{ActiveBattlePokemon, HostPokemon};

pub type BattlePlayer<ID, P, M, I, T> =
    Player<ID, ActiveBattlePokemon<ID>, HostPokemon<P, M, I>, I, T, Box<dyn BattleEndpoint<ID, T>>>;

pub struct PlayerData<ID, T> {
    pub id: ID,
    pub name: Option<String>,
    pub party: Party<SavedPokemon>,
    pub bag: SavedBag,
    pub trainer: Option<T>,
    pub settings: PlayerSettings,
    pub endpoint: Box<dyn BattleEndpoint<ID, T>>,
}

impl<
        ID,
        P: Deref<Target = Pokemon> + Clone,
        M: Deref<Target = Move> + Clone,
        I: Deref<Target = Item> + Clone,
        T,
    > BattlePlayer<ID, P, M, I, T>
{
    pub fn send(&mut self, message: ServerMessage<ID, T>) {
        self.endpoint.send(message)
    }

    pub fn receive(&mut self) -> Result<ClientMessage<ID>, Option<ReceiveError>> {
        self.endpoint.receive()
    }
}

impl<ID, T> PlayerData<ID, T> {
    pub(crate) fn init<
        P: Deref<Target = Pokemon> + Clone,
        M: Deref<Target = Move> + Clone,
        I: Deref<Target = Item> + Clone,
    >(
        self,
        random: &mut impl Rng,
        active: usize,
        pokedex: &impl Dex<Pokemon, Output = P>,
        movedex: &impl Dex<Move, Output = M>,
        itemdex: &impl Dex<Item, Output = I>,
    ) -> BattlePlayer<ID, P, M, I, T> {
        let pokemon: Party<HostPokemon<P, M, I>> = self
            .party
            .into_iter()
            .flat_map(|p| p.init(random, pokedex, movedex, itemdex))
            .map(Into::into)
            .collect();

        let mut party = PlayerParty::new(self.id, self.name, active, pokemon, self.trainer);

        for index in party.active.iter().flatten().map(ActivePokemon::index) {
            if let Some(pokemon) = party.pokemon.get_mut(index) {
                pokemon.known = true;
            }
        }

        let bag = self.bag.init(itemdex).unwrap_or_default();

        BattlePlayer {
            party,
            bag,
            settings: self.settings,
            endpoint: self.endpoint,
        }
    }
}

impl<ID: Clone, T: Clone> ClientPlayerData<ID, T> {
    pub fn new<
        'a,
        P: Deref<Target = Pokemon> + 'a + Clone,
        M: Deref<Target = Move> + 'a + Clone,
        I: Deref<Target = Item> + 'a + Clone,
        ITER: Iterator<Item = Ref<'a, BattlePlayer<ID, P, M, I, T>>>,
    >(
        data: BattleData,
        player: &BattlePlayer<ID, P, M, I, T>,
        others: ITER,
    ) -> Self
    where
        ID: 'a,
        T: 'a,
    {
        Self {
            local: PlayerParty {
                id: player.party.id().clone(),
                name: player.party.name.clone(),
                active: ActiveBattlePokemon::as_usize(&player.party.active),
                pokemon: player
                    .party
                    .pokemon
                    .iter()
                    .map(|p| &p.p.p)
                    .cloned()
                    .map(|pokemon| pokemon.uninit())
                    .collect(),
                trainer: player.party.trainer.clone(),
            },
            data,
            remotes: others.map(|player| player.party.as_remote()).collect(),
            bag: player.bag.clone().uninit(),
        }
    }
}
