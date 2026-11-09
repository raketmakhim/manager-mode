//! Transfers: players changing clubs in the summer between two seasons. One `Transfer` record covers every kind of
//! move (club to club, a youngster arriving from nowhere, a player leaving the game), and a `Window` is all the
//! transfers of one summer, ready to be applied to the `World`. Where the transfers come from is separate: the real
//! moves between two editions of the data (`transfers_to_reality`, read from the database by `source::plan_transfers`).
//! Design and data figures: `research/game-features/transfers.md`.
use crate::model::{CareerStatus, ClubId, Player, PlayerId, World};
use std::collections::{HashMap, HashSet};

/// How a player moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferKind {
    Permanent,
    /// Plays for the other club for the season. The real data shows him back at his own club a year later.
    Loan,
}

/// One player changing club. `None` is nowhere: no club before means a new player (a youngster or a free agent
/// signing), no club after means he leaves the game (retires or is released).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transfer {
    pub player: PlayerId,
    pub from: Option<ClubId>,
    pub to: Option<ClubId>,
    pub kind: TransferKind,
    /// Euros. His market value for a permanent move between clubs (the data has no real fees); 0 for the rest.
    pub fee: u32,
}

impl Transfer {
    /// True if `club` is the seller or the buyer.
    pub fn involves(&self, club: ClubId) -> bool {
        self.from == Some(club) || self.to == Some(club)
    }

    /// True for a move between two clubs, the only kind the user is asked about.
    fn is_between_clubs(&self) -> bool {
        self.from.is_some() && self.to.is_some()
    }
}

/// Where one player was at one moment, as far as transfers care.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Membership {
    pub club: Option<ClubId>,
    pub on_loan: bool,
    /// Market value in euros.
    pub value: u32,
}

/// The transfers that bring the world's clubs to where the real data (`after`) has every player. A player is moved when
/// his club in the world differs from his real new club, so a world that drifted (a club the data dropped for a
/// year, a deal the user refused) is pulled back too. Exceptions:
/// - a deal that involves `user`'s club only happens if the real player really changed club this summer (`before` to
///   `after`), so a player the user kept or turned down is not taken or sent again unasked;
/// - a player who has retired or is about to (`Player::career`) is left out;
/// - a club that has nobody in `after` is one the data dropped (for a year or more): its players stay, whatever else
///   the data says about them, so the club still has a team to play its matches.
///
/// Ordered by player id, so the same data always gives the same list.
pub fn transfers_to_reality(world: &World, before: &HashMap<PlayerId, Membership>, after: &HashMap<PlayerId, Membership>, user: Option<ClubId>) -> Vec<Transfer> {
    let covered: HashSet<ClubId> = after.values().filter_map(|m| m.club).collect();
    let mut ids: Vec<PlayerId> = after.keys().copied().chain(world.players.keys().copied()).collect();
    ids.sort_unstable();
    ids.dedup();
    ids.into_iter()
        .filter_map(|player| {
            let known = world.players.get(&player);
            if known.is_some_and(|p| p.career != CareerStatus::Playing) {
                return None; // retired (the data still lists him) or about to: he is not part of this summer's moves
            }
            let from = known.and_then(|p| p.club);
            let (was, now) = (before.get(&player), after.get(&player));
            let to = now.and_then(|m| m.club);
            if from == to {
                return None;
            }
            let really_moved = was.and_then(|m| m.club) != to;
            let involves_user = user.is_some() && (from == user || to == user);
            let dropped_club = from.is_some_and(|club| !covered.contains(&club));
            if (involves_user && !really_moved) || dropped_club {
                return None;
            }
            let loan = was.is_some_and(|m| m.on_loan) || now.is_some_and(|m| m.on_loan);
            let value = now.or(was).map_or(0, |m| m.value);
            let (kind, fee) = match (from.is_some() && to.is_some(), loan) {
                (_, true) => (TransferKind::Loan, 0),
                (true, false) => (TransferKind::Permanent, value),
                (false, false) => (TransferKind::Permanent, 0),
            };
            Some(Transfer { player, from, to, kind, fee })
        })
        .collect()
}

/// All the transfers of one summer that can be made in this world, and the user's answers to the ones that need them.
pub struct Window {
    transfers: Vec<Transfer>,
    /// The players who arrive from nowhere, by id. They are not in the world until the window is made.
    newcomers: HashMap<PlayerId, Player>,
    /// Indexes into `transfers` the user turned down.
    declined: HashSet<usize>,
}

impl Window {
    /// Keeps the transfers that can happen in `world`: the player is where the transfer says, a club that is not in
    /// the world counts as nowhere, a signing from nowhere is a free agent or a newcomer with his data, and a player of `user`'s club who
    /// would leave the game stays (the user is not asked about that, and losing a player unasked would be unfair).
    pub fn new(world: &World, transfers: Vec<Transfer>, newcomers: Vec<Player>, user: Option<ClubId>) -> Window {
        let newcomers: HashMap<PlayerId, Player> = newcomers.into_iter().map(|p| (p.id, p)).collect();
        let in_world = |club: Option<ClubId>| club.filter(|c| world.clubs.contains_key(c));
        let transfers = transfers
            .into_iter()
            .map(|t| Transfer { to: in_world(t.to), ..t })
            .filter(|t| match t.from {
                Some(_) => world.players.get(&t.player).is_some_and(|p| p.club == t.from) && !(t.to.is_none() && t.from == user),
                None => t.to.is_some() && world.players.get(&t.player).map_or(newcomers.contains_key(&t.player), |p| p.club.is_none()),
            })
            .collect();
        Window { transfers, newcomers, declined: HashSet::new() }
    }

    pub fn transfers(&self) -> &[Transfer] {
        &self.transfers
    }

    /// The transfers `club` is asked about (indexes into `transfers`): its players going to another club and other
    /// clubs' players coming to it.
    pub fn questions_for(&self, club: ClubId) -> Vec<usize> {
        (0..self.transfers.len()).filter(|&i| self.transfers[i].involves(club) && self.transfers[i].is_between_clubs()).collect()
    }

    /// Turns a transfer down (true) or goes back on it (false). The player then stays where he is.
    pub fn set_declined(&mut self, index: usize, declined: bool) {
        if declined {
            self.declined.insert(index);
        } else {
            self.declined.remove(&index);
        }
    }

    pub fn is_declined(&self, index: usize) -> bool {
        self.declined.contains(&index)
    }
}

impl World {
    /// Makes every transfer of the window that was not declined. Squads end up best first, like a loaded edition's.
    pub fn make_transfers(&mut self, mut window: Window) {
        let mut changed: HashSet<ClubId> = HashSet::new();
        for (i, t) in window.transfers.iter().enumerate() {
            if window.declined.contains(&i) {
                continue;
            }
            if t.from.is_none() {
                if let Some(newcomer) = window.newcomers.remove(&t.player) {
                    self.players.insert(newcomer.id, newcomer);
                }
            }
            let Some(player) = self.players.get_mut(&t.player) else { continue };
            player.club = t.to;
            if let Some(from) = t.from.and_then(|c| self.clubs.get_mut(&c)) {
                from.squad.retain(|&id| id != t.player);
            }
            if let Some(to) = t.to.and_then(|c| self.clubs.get_mut(&c)) {
                to.squad.push(t.player);
            }
            changed.extend(t.from.into_iter().chain(t.to));
        }
        for club in changed {
            let players = &self.players;
            self.clubs.get_mut(&club).expect("a changed club exists").squad.sort_by_key(|id| (std::cmp::Reverse(players[id].overall), *id));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{add_club_with_eleven, player};

    fn at(club: Option<ClubId>) -> Membership {
        Membership { club, on_loan: false, value: 1_000_000 }
    }

    /// Two clubs of eleven: club 1 has players 100 to 110, club 2 has 200 to 210.
    fn world() -> World {
        let mut w = World::new(15, 2014);
        add_club_with_eleven(&mut w, 1, 7, 60);
        add_club_with_eleven(&mut w, 2, 7, 60);
        w
    }

    fn moves(player: PlayerId, from: Option<ClubId>, to: Option<ClubId>) -> Transfer {
        Transfer { player, from, to, kind: TransferKind::Permanent, fee: 0 }
    }

    fn newcomer(id: PlayerId, club: ClubId) -> Player {
        Player { club: Some(club), ..player(id, 70, &[crate::model::Position::St]) }
    }

    /// Where every player of the world is now, as a "before" and "after" the test can change.
    fn memberships(w: &World) -> HashMap<PlayerId, Membership> {
        w.players.values().map(|p| (p.id, at(p.club))).collect()
    }

    #[test]
    fn transfers_to_reality_find_who_moved_left_or_arrived() {
        let w = world();
        let before = memberships(&w);
        let mut after = before.clone();
        after.insert(100, at(Some(2))); // moved
        after.remove(&101); // gone from the data
        after.insert(500, at(Some(2))); // new
        let moves = transfers_to_reality(&w, &before, &after, None);
        let summary: Vec<(PlayerId, Option<ClubId>, Option<ClubId>)> = moves.iter().map(|t| (t.player, t.from, t.to)).collect();
        assert_eq!(summary, vec![(100, Some(1), Some(2)), (101, Some(1), None), (500, None, Some(2))]);
        assert_eq!(moves[0].fee, 1_000_000, "a permanent move between clubs costs his value");
        assert!(moves[1..].iter().all(|t| t.fee == 0), "nothing is paid when there is no club on one side");
    }

    #[test]
    fn a_loan_costs_nothing_whichever_way_it_goes() {
        let w = world();
        let before = memberships(&w);
        let mut after = before.clone();
        after.insert(100, Membership { on_loan: true, ..at(Some(2)) });
        let mut loaned_before = before.clone();
        loaned_before.insert(200, Membership { on_loan: true, ..at(Some(2)) });
        after.insert(200, at(Some(1)));
        for t in transfers_to_reality(&w, &loaned_before, &after, None) {
            assert_eq!((t.kind, t.fee), (TransferKind::Loan, 0));
        }
    }

    #[test]
    fn a_player_who_has_retired_or_is_about_to_is_not_moved_whatever_the_data_says() {
        for career in [CareerStatus::Retired, CareerStatus::Retiring] {
            let mut w = world();
            w.players.get_mut(&100).unwrap().career = career;
            let before = memberships(&w);
            let mut after = before.clone();
            after.insert(100, at(Some(2)));
            assert!(transfers_to_reality(&w, &before, &after, None).is_empty(), "{career:?}");
        }
    }

    #[test]
    fn a_club_the_data_dropped_keeps_its_players_even_if_the_data_has_them_elsewhere() {
        let w = world();
        let before = memberships(&w);
        let after: HashMap<PlayerId, Membership> = before.iter().filter(|(_, m)| m.club == Some(1)).map(|(&id, &m)| (id, m)).collect(); // nobody at club 2
        let mut after = after;
        after.insert(200, at(Some(1))); // the data has one of them at club 1
        assert!(transfers_to_reality(&w, &before, &after, None).is_empty(), "club 2's players neither leave nor move");
    }

    #[test]
    fn a_world_that_drifted_is_pulled_back_except_for_the_users_own_deals() {
        let w = world();
        let before = memberships(&w); // the data had player 100 at club 1, where the world has him
        let mut after = before.clone();
        after.insert(100, at(Some(2)));
        let mut drifted = before.clone(); // the data had him at club 2 already, but the world kept him at club 1
        drifted.insert(100, at(Some(2)));
        let pulled = transfers_to_reality(&w, &drifted, &after, None);
        assert_eq!(pulled.len(), 1, "another club's player is moved to where the data has him");
        assert!(transfers_to_reality(&w, &drifted, &after, Some(1)).is_empty(), "a player the user kept is not sold unasked");
        assert_eq!(transfers_to_reality(&w, &before, &after, Some(1)).len(), 1, "a real move is still offered");
    }

    #[test]
    fn only_transfers_that_can_happen_are_kept() {
        let w = world();
        let transfers = vec![
            moves(100, Some(1), Some(2)),  // fine
            moves(101, Some(2), Some(1)),  // not at club 2
            moves(102, Some(1), Some(99)), // club 99 is not in the world: he leaves the game, so the user's player stays
            moves(103, Some(1), None),     // the user's player would leave the game: he stays
            moves(500, None, Some(2)),     // a newcomer with data
            moves(501, None, Some(2)),     // a newcomer without data
        ];
        let window = Window::new(&w, transfers, vec![newcomer(500, 2)], Some(1));
        let kept: Vec<(PlayerId, Option<ClubId>)> = window.transfers().iter().map(|t| (t.player, t.to)).collect();
        assert_eq!(kept, vec![(100, Some(2)), (500, Some(2))]);

        let others = Window::new(&w, vec![moves(102, Some(1), Some(99)), moves(103, Some(1), None)], vec![], None);
        assert_eq!(others.transfers().len(), 2, "another club's player does leave");
    }

    #[test]
    fn only_moves_between_clubs_are_asked_about() {
        let w = world();
        let window = Window::new(&w, vec![moves(100, Some(1), Some(2)), moves(101, Some(2), None), moves(500, None, Some(1))], vec![newcomer(500, 1)], None);
        assert_eq!(window.questions_for(1), vec![0]);
        assert_eq!(window.questions_for(2), vec![0], "the buyer is asked too");
    }

    #[test]
    fn making_transfers_keeps_squads_and_players_in_agreement() {
        let mut w = world();
        let mut window = Window::new(&w, vec![moves(100, Some(1), Some(2)), moves(200, Some(2), Some(1)), moves(101, Some(1), None), moves(500, None, Some(2))], vec![newcomer(500, 2)], None);
        window.set_declined(1, true);
        w.make_transfers(window);

        assert_eq!(w.players[&100].club, Some(2));
        assert_eq!(w.players[&200].club, Some(2), "a declined transfer leaves him where he was");
        assert_eq!(w.players[&101].club, None);
        assert_eq!(w.players[&500].club, Some(2));
        assert_eq!((w.clubs[&1].squad.len(), w.clubs[&2].squad.len()), (9, 13));
        for c in w.clubs.values() {
            assert!(c.squad.iter().all(|id| w.players[id].club == Some(c.id)));
            assert!(c.squad.windows(2).all(|pair| w.players[&pair[0]].overall >= w.players[&pair[1]].overall), "best first");
        }
        let in_squads: usize = w.clubs.values().map(|c| c.squad.len()).sum();
        assert_eq!(in_squads, w.players.values().filter(|p| p.club.is_some()).count());
    }
}
