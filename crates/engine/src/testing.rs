//! Small builders for tests, so no test writes out a whole `Player` or `Club` by hand.
use crate::attrs::Attr;
use crate::lineup::{Formation, Selection};
use crate::model::{CareerStatus, Club, ClubId, CompetitionId, Player, PlayerId, Position, World};

/// A player with the given overall and listed positions; everything else is neutral.
pub fn player(id: PlayerId, overall: u8, positions: &[Position]) -> Player {
    Player {
        id,
        name: format!("P{id}"),
        birth_date: "1995-01-01".into(),
        positions: positions.to_vec(),
        attrs: Default::default(),
        overall,
        potential: overall,
        club: None,
        wage: 0,
        contract_until: None,
        value: 0,
        form: 50,
        fitness: 100,
        fitness_day: 0,
        matches_in_a_row: 0,
        last_match_day: 0,
        real_change: 0,
        xp: 0,
        career: CareerStatus::Playing,
    }
}

/// A player like `player`, with every attribute set to 50 except the ones given.
pub fn player_with_attrs(id: PlayerId, overall: u8, positions: &[Position], attrs: &[(Attr, u8)]) -> Player {
    let mut p = player(id, overall, positions);
    for &attr in Attr::ALL {
        p.attrs.set(attr, 50);
    }
    for &(attr, value) in attrs {
        p.attrs.set(attr, value);
    }
    p
}

/// A club in `league` with the given squad.
pub fn club(id: ClubId, league: CompetitionId, squad: Vec<PlayerId>) -> Club {
    Club { id, name: format!("C{id}"), league, squad, budget: None, selection: Selection::default(), formation: Formation::default(), formation_set: false }
}

/// Adds a club of eleven natural 4-4-2 players, all rated `overall`, to the world. Their ids are `club_id * 100 + 0..11`.
pub fn add_club_with_eleven(world: &mut World, club_id: ClubId, league: CompetitionId, overall: u8) {
    let mut squad = vec![];
    for (i, position) in Formation::FourFourTwo.slots().into_iter().enumerate() {
        let mut p = player(club_id * 100 + i as u32, overall, &[position]);
        p.club = Some(club_id);
        squad.push(p.id);
        world.players.insert(p.id, p);
    }
    world.clubs.insert(club_id, club(club_id, league, squad));
}
