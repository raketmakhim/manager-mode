//! Lineups: a formation is eleven positions, and picking a lineup puts a player in each one, then seven substitutes
//! on the bench. The picking is a pure function; `World` only stores the result and reads it back for the match
//! simulation.
use crate::model::{ClubId, Line, Player, PlayerId, Position, World};
use crate::overall::weights;

/// The eleven players of a match, one per slot of the club's formation (`Formation::slots`), or `None` for a slot
/// nobody fills (a club with fewer than eleven players).
pub type Lineup = [Option<PlayerId>; 11];

/// Substitutes on the bench for a match.
pub const BENCH_SIZE: usize = 7;
/// The substitutes, best first as picked; `None` for an empty place.
pub type Bench = [Option<PlayerId>; BENCH_SIZE];

/// Rating points lost when a player plays a position that is not one of his listed positions. The attribute-based
/// rating already counts what he can do there; this is for being unused to it.
const UNFAMILIAR_PENALTY: u8 = 3;
/// Used instead of the attributes when the data lacks one of them (FIFA 15): rating points lost for a position on
/// the same line as one of his, and for a position on another line.
const SAME_LINE_PENALTY: u8 = 5;
const OTHER_LINE_PENALTY: u8 = 15;
/// Rating points lost when a goalkeeper plays outfield or an outfielder in goal: practically never worth it.
const KEEPER_MISMATCH_PENALTY: u8 = 70;

/// How a team lines up. Each is a list of eleven positions, goalkeeper first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Formation {
    #[default]
    FourFourTwo,
    FourThreeThree,
    FourTwoThreeOne,
    ThreeFiveTwo,
    FourOneFourOne,
    FourFourOneOne,
    /// The diamond: a holding midfielder, two central ones and an attacking one behind two strikers.
    FourOneTwoOneTwo,
    ThreeFourTwoOne,
    ThreeFourThree,
    FiveThreeTwo,
}

impl Formation {
    /// Every formation, in the order they are shown.
    pub const ALL: &'static [Formation] = &[
        Formation::FourFourTwo,
        Formation::FourThreeThree,
        Formation::FourTwoThreeOne,
        Formation::ThreeFiveTwo,
        Formation::FourOneFourOne,
        Formation::FourFourOneOne,
        Formation::FourOneTwoOneTwo,
        Formation::ThreeFourTwoOne,
        Formation::ThreeFourThree,
        Formation::FiveThreeTwo,
    ];

    /// "4-4-2".
    pub fn name(self) -> &'static str {
        match self {
            Formation::FourFourTwo => "4-4-2",
            Formation::FourThreeThree => "4-3-3",
            Formation::FourTwoThreeOne => "4-2-3-1",
            Formation::ThreeFiveTwo => "3-5-2",
            Formation::FourOneFourOne => "4-1-4-1",
            Formation::FourFourOneOne => "4-4-1-1",
            Formation::FourOneTwoOneTwo => "4-1-2-1-2",
            Formation::ThreeFourTwoOne => "3-4-2-1",
            Formation::ThreeFourThree => "3-4-3",
            Formation::FiveThreeTwo => "5-3-2",
        }
    }

    /// The formation as rows from the goalkeeper up the pitch, each row left to right. This is the one definition:
    /// the slots and the drawing of the pitch both come from it.
    pub fn rows(self) -> &'static [&'static [Position]] {
        use Position::*;
        match self {
            Formation::FourFourTwo => &[&[Gk], &[Lb, Cb, Cb, Rb], &[Lm, Cm, Cm, Rm], &[St, St]],
            Formation::FourThreeThree => &[&[Gk], &[Lb, Cb, Cb, Rb], &[Cm, Cdm, Cm], &[Lw, St, Rw]],
            Formation::FourTwoThreeOne => &[&[Gk], &[Lb, Cb, Cb, Rb], &[Cdm, Cdm], &[Lm, Cam, Rm], &[St]],
            Formation::ThreeFiveTwo => &[&[Gk], &[Cb, Cb, Cb], &[Lwb, Cdm, Cm, Cam, Rwb], &[St, St]],
            Formation::FourOneFourOne => &[&[Gk], &[Lb, Cb, Cb, Rb], &[Cdm], &[Lm, Cm, Cm, Rm], &[St]],
            Formation::FourFourOneOne => &[&[Gk], &[Lb, Cb, Cb, Rb], &[Lm, Cm, Cm, Rm], &[Cam], &[St]],
            Formation::FourOneTwoOneTwo => &[&[Gk], &[Lb, Cb, Cb, Rb], &[Cdm], &[Cm, Cm], &[Cam], &[St, St]],
            Formation::ThreeFourTwoOne => &[&[Gk], &[Cb, Cb, Cb], &[Lwb, Cm, Cm, Rwb], &[Cam, Cam], &[St]],
            Formation::ThreeFourThree => &[&[Gk], &[Cb, Cb, Cb], &[Lwb, Cm, Cm, Rwb], &[Lw, St, Rw]],
            Formation::FiveThreeTwo => &[&[Gk], &[Lwb, Cb, Cb, Cb, Rwb], &[Cm, Cdm, Cm], &[St, St]],
        }
    }

    /// The eleven positions in row order (so the goalkeeper is slot 0). A slot's number is its place in this list.
    pub fn slots(self) -> [Position; 11] {
        let flat: Vec<Position> = self.rows().iter().flat_map(|row| row.iter().copied()).collect();
        flat.try_into().expect("every formation has eleven positions")
    }
}

/// How well a position suits a player, by his listed positions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Familiarity {
    /// One of his listed positions.
    Natural,
    /// Not listed, but on the same line (a centre-back at right-back).
    SameLine,
    /// On another line (a midfielder at centre-back).
    OtherLine,
}

/// True if the two positions are the same job: a centre-forward and a striker are listed apart in the data (FIFA
/// lists some forwards only as CF) but play the same role, so neither counts as out of position in the other.
fn same_role(a: Position, b: Position) -> bool {
    a == b || matches!((a, b), (Position::St, Position::Cf) | (Position::Cf, Position::St))
}

/// How well `slot` suits the player.
pub fn familiarity(player: &Player, slot: Position) -> Familiarity {
    if player.positions.iter().any(|&p| same_role(p, slot)) {
        Familiarity::Natural
    } else if player.positions.iter().any(|p| p.line() == slot.line()) {
        Familiarity::SameLine
    } else {
        Familiarity::OtherLine
    }
}

/// What the position's overall formula gives for this player's attributes, or `None` if one of them is missing.
fn formula_at(player: &Player, position: Position) -> Option<f32> {
    weights(position).iter().map(|&(attr, weight)| Some(weight * player.attrs[attr]? as f32)).sum()
}

/// The listed position his overall belongs to: the one whose formula gives the most for his attributes (the first
/// listed on a tie or when attributes are missing). The data's overall is for his best position, which is not always
/// the first listed.
fn overall_position(player: &Player) -> Option<Position> {
    let first = *player.positions.first()?;
    let best = player.positions.iter().filter_map(|&pos| Some((pos, formula_at(player, pos)?))).fold(None, |best: Option<(Position, f32)>, now| match best {
        Some(b) if b.1 >= now.1 => Some(b),
        _ => Some(now),
    });
    Some(best.map_or(first, |(pos, _)| pos))
}

/// A player's rating when he plays `slot` (0 to 99). At the position his overall belongs to (`overall_position`) it is
/// his overall. At another position it is his overall plus the difference between that position's formula and his
/// own position's, worked out from his own attributes, minus a small penalty if the position is not one he is listed for.
pub fn rating_at(player: &Player, slot: Position) -> u8 {
    let Some(main) = overall_position(player) else {
        return player.overall.saturating_sub(if slot == Position::Gk { KEEPER_MISMATCH_PENALTY } else { OTHER_LINE_PENALTY });
    };
    if (slot == Position::Gk) != (main == Position::Gk) {
        return player.overall.saturating_sub(KEEPER_MISMATCH_PENALTY);
    }
    let slot = if same_role(main, slot) { main } else { slot }; // his own role: his overall
    let familiarity = familiarity(player, slot);
    let rating = match (formula_at(player, slot), formula_at(player, main)) {
        (Some(there), Some(here)) => {
            let unfamiliar = if familiarity == Familiarity::Natural { 0.0 } else { UNFAMILIAR_PENALTY as f32 };
            player.overall as f32 + there - here - unfamiliar
        }
        _ => {
            let penalty = match familiarity {
                Familiarity::Natural => 0,
                Familiarity::SameLine => SAME_LINE_PENALTY,
                Familiarity::OtherLine => OTHER_LINE_PENALTY,
            };
            player.overall as f32 - penalty as f32
        }
    };
    rating.round().clamp(0.0, 99.0) as u8
}

/// The best eleven for a formation. Takes the highest rating of any (player, slot) pair, fills that slot with
/// that player, and repeats; ties go to the lower player id and the earlier slot, so the result never varies.
// shortcut: this greedy pick is not always the best total assignment; upgrade to an optimal assignment (Hungarian
// method) if lineups look wrong.
pub fn pick_lineup(squad: &[&Player], formation: Formation) -> Lineup {
    pick_by(squad, formation, rating_at)
}

/// `pick_lineup` ranking each player in each slot by `rate`.
fn pick_by(squad: &[&Player], formation: Formation, rate: impl Fn(&Player, Position) -> u8) -> Lineup {
    let slots = formation.slots();
    let mut pairs: Vec<(u8, usize, PlayerId, usize)> = vec![]; // (rating, index in squad, player id, slot)
    for (i, player) in squad.iter().enumerate() {
        for (slot, &position) in slots.iter().enumerate() {
            pairs.push((rate(player, position), i, player.id, slot));
        }
    }
    pairs.sort_by_key(|&(rating, _, id, slot)| (std::cmp::Reverse(rating), id, slot));
    let (mut lineup, mut used) = ([None; 11], vec![false; squad.len()]);
    for (_, i, id, slot) in pairs {
        if lineup[slot].is_none() && !used[i] {
            lineup[slot] = Some(id);
            used[i] = true;
        }
    }
    lineup
}

/// The formation whose best eleven has the highest total rating. A tie goes to the earlier one in `Formation::ALL`, so
/// the usual 4-4-2 is kept unless another shape is really better.
pub fn best_formation(squad: &[&Player]) -> Formation {
    let total = |formation: Formation| -> u32 {
        let slots = formation.slots();
        let lineup = pick_lineup(squad, formation);
        let rating = |slot: usize| lineup[slot].and_then(|id| squad.iter().find(|p| p.id == id)).map_or(0, |p| rating_at(p, slots[slot]) as u32);
        (0..11).map(rating).sum()
    };
    Formation::ALL.iter().copied().rev().max_by_key(|&f| total(f)).unwrap_or_default()
}

/// The substitutes: a back-up goalkeeper, then the best remaining player of each line (defence, midfield, attack),
/// then the best of whoever is left, up to `BENCH_SIZE`. Best means highest overall, ties to the lower id.
pub fn pick_bench(squad: &[&Player], lineup: &Lineup) -> Bench {
    let mut rest: Vec<&Player> = squad.iter().copied().filter(|p| !lineup.contains(&Some(p.id))).collect();
    rest.sort_by_key(|p| (std::cmp::Reverse(p.overall), p.id));
    let main_line = |p: &Player| p.positions.first().map(|pos| pos.line());
    let mut bench: Vec<PlayerId> = vec![];
    for line in [Line::Goalkeeper, Line::Defence, Line::Midfield, Line::Attack] {
        if let Some(at) = rest.iter().position(|&p| main_line(p) == Some(line)) {
            bench.push(rest.remove(at).id);
        }
    }
    bench.extend(rest.iter().map(|p| p.id));
    let mut result = [None; BENCH_SIZE];
    for (place, id) in result.iter_mut().zip(bench) {
        *place = Some(id);
    }
    result
}

/// Where a player can be in the squad's selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spot {
    /// A starting slot, by index into `Formation::slots`.
    Starter(usize),
    /// A place on the bench.
    Sub(usize),
    /// Neither starting nor on the bench.
    Reserve(PlayerId),
}

/// Who starts and who is on the bench. Everyone else in the squad is a reserve.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    pub lineup: Lineup,
    pub bench: Bench,
}

impl Selection {
    /// Nobody picked yet.
    pub fn is_empty(&self) -> bool {
        self.lineup.iter().all(Option::is_none)
    }

    /// The player at a spot (a reserve is always himself), or `None` for an empty place.
    pub fn player_at(&self, spot: Spot) -> Option<PlayerId> {
        match spot {
            Spot::Starter(i) => self.lineup[i],
            Spot::Sub(i) => self.bench[i],
            Spot::Reserve(id) => Some(id),
        }
    }

    /// Puts a player (or nobody) in a spot. Reserves are not stored, so this does nothing for one.
    fn put(&mut self, spot: Spot, player: Option<PlayerId>) {
        match spot {
            Spot::Starter(i) => self.lineup[i] = player,
            Spot::Sub(i) => self.bench[i] = player,
            Spot::Reserve(_) => {}
        }
    }

    /// The two spots exchange players. Swapping a starter with a reserve sends the starter to the reserves.
    pub fn swap(&mut self, a: Spot, b: Spot) {
        let (in_a, in_b) = (self.player_at(a), self.player_at(b));
        self.put(a, in_b);
        self.put(b, in_a);
    }

    /// True if everyone chosen is in `squad` and nobody is chosen twice.
    fn is_valid_for(&self, squad: &[PlayerId]) -> bool {
        let chosen: Vec<PlayerId> = self.lineup.iter().chain(&self.bench).flatten().copied().collect();
        chosen.iter().all(|p| squad.contains(p)) && chosen.iter().enumerate().all(|(i, p)| !chosen[..i].contains(p))
    }
}

impl World {
    /// Gives every club that has not had its formation chosen by the user the formation that suits its squad best.
    /// Call it when squads or ratings change (a new season); the match simulation does not do it, to stay fast.
    pub fn choose_formations(&mut self) {
        let unset: Vec<ClubId> = self.clubs.values().filter(|c| !c.formation_set).map(|c| c.id).collect();
        for club in unset {
            self.choose_formation(club);
        }
    }

    /// Gives one club the formation that suits its squad best, whether or not the user had set one.
    pub fn choose_formation(&mut self, club: ClubId) {
        let squad: Vec<&Player> = self.clubs[&club].squad.iter().map(|p| &self.players[p]).collect();
        let formation = best_formation(&squad);
        self.clubs.get_mut(&club).expect("club exists").formation = formation;
    }

    /// The selection the user set for the club, if there is one and it is still valid (everyone still in the squad,
    /// nobody twice). An empty or out-of-date one counts as not set.
    fn set_selection(&self, club: ClubId) -> Option<Selection> {
        let c = &self.clubs[&club];
        (!c.selection.is_empty() && c.selection.is_valid_for(&c.squad)).then_some(c.selection)
    }

    /// True if the user has set a selection that is still valid. False means the game picks before each match.
    pub fn has_set_selection(&self, club: ClubId) -> bool {
        self.set_selection(club).is_some()
    }

    /// The club's lineup and bench for its next match: the one the user set if it is still valid, otherwise a
    /// freshly picked one. Not stored: a club that never sets one is re-picked each time, so it follows its squad.
    pub fn selection_of(&self, club: ClubId) -> Selection {
        self.set_selection(club).unwrap_or_else(|| self.picked_selection(club))
    }

    /// Just the lineup of `selection_of`, for callers that do not need the bench.
    pub fn lineup_of(&self, club: ClubId) -> Lineup {
        match self.set_selection(club) {
            Some(selection) => selection.lineup,
            None => {
                let c = &self.clubs[&club];
                let squad: Vec<&Player> = c.squad.iter().map(|p| &self.players[p]).collect();
                pick_lineup(&squad, c.formation)
            }
        }
    }

    /// The best lineup and bench for the club's formation, without storing them.
    pub fn picked_selection(&self, club: ClubId) -> Selection {
        let c = &self.clubs[&club];
        let squad: Vec<&Player> = c.squad.iter().map(|p| &self.players[p]).collect();
        // A player who is not fully fit starts the match tired, so the game picks by what he is worth today: the
        // better of two close players rests when he is worn out.
        let lineup = pick_by(&squad, c.formation, |p, slot| rating_at(p, slot).saturating_sub(self.rating_cost_of_fitness(p.fitness)));
        Selection { lineup, bench: pick_bench(&squad, &lineup) }
    }

    /// The rating of the player in each slot, for the match simulation (0 for an empty slot).
    pub fn lineup_ratings(&self, club: ClubId) -> [u8; 11] {
        let slots = self.clubs[&club].formation.slots();
        let lineup = self.lineup_of(club);
        std::array::from_fn(|i| lineup[i].map_or(0, |p| rating_at(&self.players[&p], slots[i])))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attrs::Attr;
    use crate::testing::{club, player, player_with_attrs};
    use Position::*;

    #[test]
    fn the_game_rests_a_worn_out_player_when_a_fresh_one_is_nearly_as_good() {
        let mut w = crate::model::World::new(15, 2014);
        crate::testing::add_club_with_eleven(&mut w, 1, 1, 60);
        let mut star = player(999, 61, &[Cm]);
        star.club = Some(1);
        w.players.insert(999, star);
        w.clubs.get_mut(&1).unwrap().squad.push(999);
        assert!(w.picked_selection(1).lineup.contains(&Some(999)), "fully fit, the 61 plays");
        w.players.get_mut(&999).unwrap().fitness = 30;
        assert!(!w.picked_selection(1).lineup.contains(&Some(999)), "worn out, he rests for a 60");
    }

    #[test]
    fn every_formation_is_eleven_players_in_rows_with_the_goalkeeper_alone_first() {
        for &f in Formation::ALL {
            assert_eq!(f.rows().iter().map(|row| row.len()).sum::<usize>(), 11, "{}", f.name());
            assert_eq!(f.rows()[0], &[Gk], "{}", f.name());
            // The name counts the outfield rows, e.g. 4-2-3-1 has 4, 2, 3 and 1 players.
            let counts: Vec<String> = f.rows()[1..].iter().map(|row| row.len().to_string()).collect();
            assert_eq!(counts.join("-"), f.name());
        }
    }

    #[test]
    fn every_formation_has_eleven_slots_with_one_goalkeeper_first() {
        for &f in Formation::ALL {
            let slots = f.slots();
            assert_eq!(slots[0], Gk, "{}", f.name());
            assert_eq!(slots.iter().filter(|&&p| p == Gk).count(), 1, "{}", f.name());
        }
    }

    #[test]
    fn a_player_listed_first_at_a_position_that_does_not_fit_his_attributes_is_not_inflated_at_his_best_one() {
        // Listed RM first, but his attributes are a full-back's: the data's overall belongs to RB.
        let tackler = [(Attr::Marking, 85), (Attr::StandingTackle, 85), (Attr::SlidingTackle, 85), (Attr::Interceptions, 85), (Attr::Finishing, 30)];
        let p = player_with_attrs(1, 81, &[Rm, Rb], &tackler);
        assert_eq!(rating_at(&p, Rb), 81, "at the position his overall belongs to he is worth his overall");
        assert!(rating_at(&p, Rm) < 81);
    }

    #[test]
    fn without_attributes_playing_out_of_position_costs_a_flat_penalty() {
        let centre_back = player(1, 80, &[Cb]);
        assert_eq!(rating_at(&centre_back, Cb), 80);
        assert_eq!(rating_at(&centre_back, Rb), 75, "same line");
        assert_eq!(rating_at(&centre_back, St), 65, "another line");
        assert!(rating_at(&centre_back, Gk) < 20, "an outfielder in goal is hopeless");
        assert!(rating_at(&player(2, 80, &[Gk]), Cb) < 20, "so is a keeper outfield");
        assert_eq!(rating_at(&player(3, 80, &[St, Cam]), Cm), 75, "any listed position on the line counts");
    }

    #[test]
    fn with_attributes_the_rating_follows_what_the_position_needs() {
        // A striker with great finishing and poor tackling.
        let striker = player_with_attrs(1, 80, &[St], &[(Attr::Finishing, 95), (Attr::StandingTackle, 30), (Attr::Marking, 30)]);
        assert_eq!(rating_at(&striker, St), 80, "at his own position it is exactly his overall");
        assert!(rating_at(&striker, Cb) < 70, "a striker with no tackling is much worse at centre-back");
        // The same overall but a defender's attributes: good at centre-back, weak up front.
        let defender = player_with_attrs(2, 80, &[Cb], &[(Attr::Finishing, 30), (Attr::StandingTackle, 90), (Attr::Marking, 90)]);
        assert_eq!(rating_at(&defender, Cb), 80);
        assert!(rating_at(&defender, St) < rating_at(&defender, Cb) - 10);
    }

    #[test]
    fn a_listed_second_position_has_no_familiarity_penalty_but_an_unlisted_one_has() {
        let attrs = [(Attr::Finishing, 70), (Attr::Dribbling, 70)];
        let listed = player_with_attrs(1, 80, &[St, Lw], &attrs);
        let unlisted = player_with_attrs(2, 80, &[St], &attrs);
        assert_eq!(rating_at(&unlisted, Lw) + UNFAMILIAR_PENALTY, rating_at(&listed, Lw));
    }

    #[test]
    fn a_natural_squad_gets_its_best_natural_player_in_each_slot() {
        // Two players per position: the better one must start.
        let mut players = vec![];
        for (i, position) in Formation::FourFourTwo.slots().into_iter().enumerate() {
            players.push(player(100 + i as u32, 80, &[position]));
            players.push(player(200 + i as u32, 60, &[position]));
        }
        let squad: Vec<&Player> = players.iter().collect();
        let lineup = pick_lineup(&squad, Formation::FourFourTwo);
        assert!(lineup.iter().flatten().all(|&id| (100..200).contains(&id)), "{lineup:?}");
    }

    #[test]
    fn a_lineup_has_no_player_twice_and_puts_the_keeper_in_goal() {
        let mut players = vec![player(1, 70, &[Gk]), player(2, 90, &[Gk])];
        for id in 3..25 {
            let position = [Cb, Cm, St][id as usize % 3];
            players.push(player(id, 60 + id as u8 % 10, &[position]));
        }
        let squad: Vec<&Player> = players.iter().collect();
        for &formation in Formation::ALL {
            let lineup = pick_lineup(&squad, formation);
            let picked: Vec<PlayerId> = lineup.iter().flatten().copied().collect();
            assert_eq!(picked.len(), 11, "{}", formation.name());
            let mut distinct = picked.clone();
            distinct.sort();
            distinct.dedup();
            assert_eq!(distinct.len(), 11, "{}: a player appears twice", formation.name());
            assert_eq!(lineup[0], Some(2), "the better keeper starts");
        }
    }

    #[test]
    fn a_short_squad_leaves_slots_empty_and_picking_is_repeatable() {
        let players: Vec<Player> = (1..=7).map(|id| player(id, 70, &[Cm])).collect();
        let squad: Vec<&Player> = players.iter().collect();
        let lineup = pick_lineup(&squad, Formation::FourFourTwo);
        assert_eq!(lineup.iter().flatten().count(), 7);
        assert_eq!(lineup, pick_lineup(&squad, Formation::FourFourTwo));
    }

    #[test]
    fn the_bench_has_a_back_up_keeper_and_cover_for_every_line_and_nobody_from_the_lineup() {
        let mut players = vec![player(1, 70, &[Gk]), player(2, 65, &[Gk])];
        for id in 3..30 {
            let position = [Cb, Cm, St][id as usize % 3];
            players.push(player(id, 50 + id as u8 % 20, &[position]));
        }
        let squad: Vec<&Player> = players.iter().collect();
        let lineup = pick_lineup(&squad, Formation::FourFourTwo);
        let bench = pick_bench(&squad, &lineup);
        let on_bench: Vec<&Player> = bench.iter().flatten().map(|id| squad.iter().find(|p| p.id == *id).copied().unwrap()).collect();
        assert_eq!(on_bench.len(), BENCH_SIZE);
        assert!(on_bench.iter().all(|p| !lineup.contains(&Some(p.id))), "a starter is also on the bench");
        for line in [Line::Goalkeeper, Line::Defence, Line::Midfield, Line::Attack] {
            assert!(on_bench.iter().any(|p| p.positions[0].line() == line), "no cover for {line:?}");
        }
    }

    #[test]
    fn swapping_exchanges_two_spots_and_sends_a_swapped_out_starter_to_the_reserves() {
        let mut s = Selection::default();
        s.lineup[0] = Some(1);
        s.lineup[1] = Some(2);
        s.bench[0] = Some(3);

        s.swap(Spot::Starter(0), Spot::Starter(1)); // two starters trade slots
        assert_eq!((s.lineup[0], s.lineup[1]), (Some(2), Some(1)));

        s.swap(Spot::Starter(0), Spot::Sub(0)); // a starter and a sub
        assert_eq!((s.lineup[0], s.bench[0]), (Some(3), Some(2)));

        s.swap(Spot::Sub(0), Spot::Reserve(9)); // the sub goes to the reserves, a reserve takes his place
        assert_eq!(s.bench[0], Some(9));
        assert!(!s.lineup.contains(&Some(2)) && !s.bench.contains(&Some(2)));

        s.swap(Spot::Starter(5), Spot::Sub(0)); // an empty slot takes the player and leaves the bench place empty
        assert_eq!((s.lineup[5], s.bench[0]), (Some(9), None));

        let before = s;
        s.swap(Spot::Reserve(4), Spot::Reserve(5)); // two reserves: nothing to do
        assert_eq!(s, before);
    }

    #[test]
    fn a_set_selection_is_used_until_someone_leaves_the_squad() {
        let mut w = World::new(0, 2014);
        let mut squad = vec![];
        for (i, position) in Formation::FourFourTwo.slots().into_iter().enumerate() {
            let id = i as u32 + 1;
            w.players.insert(id, player(id, 70, &[position]));
            squad.push(id);
        }
        w.clubs.insert(1, club(1, 1, squad));

        let mut chosen = w.picked_selection(1);
        chosen.swap(Spot::Starter(9), Spot::Starter(10)); // the user swaps the two strikers
        w.clubs.get_mut(&1).unwrap().selection = chosen;
        assert_eq!(w.selection_of(1), chosen);

        w.clubs.get_mut(&1).unwrap().squad.retain(|&p| p != 10); // a lineup player is sold
        assert_ne!(w.selection_of(1), chosen, "falls back to a fresh pick");
        assert!(!w.selection_of(1).lineup.contains(&Some(10)));
    }

    #[test]
    fn an_automatic_club_follows_its_ratings_but_a_set_selection_does_not() {
        let mut w = World::new(0, 2014);
        let mut squad = vec![];
        for (i, position) in Formation::FourFourTwo.slots().into_iter().enumerate() {
            let id = i as u32 + 1;
            w.players.insert(id, player(id, 70, &[position]));
            squad.push(id);
        }
        w.players.insert(50, player(50, 90, &[St])); // a star striker on the bench
        squad.push(50);
        w.clubs.insert(1, club(1, 1, squad));
        assert!(w.lineup_of(1).contains(&Some(50)), "the star starts");
        assert!(!w.has_set_selection(1));

        let chosen = w.picked_selection(1);
        w.clubs.get_mut(&1).unwrap().selection = chosen;
        assert!(w.has_set_selection(1));
        w.players.get_mut(&50).unwrap().overall = 10; // his rating collapses at the new season
        assert!(w.lineup_of(1).contains(&Some(50)), "a set selection stays as the user left it");

        w.clubs.get_mut(&1).unwrap().selection = Selection::default(); // back to automatic
        assert!(!w.lineup_of(1).contains(&Some(50)), "automatic picks again and drops him");
    }

    #[test]
    fn every_position_is_on_one_line_and_the_lines_have_the_expected_sizes() {
        let count = |line| Position::ALL.iter().filter(|p| p.line() == line).count();
        assert_eq!((count(Line::Goalkeeper), count(Line::Defence), count(Line::Midfield), count(Line::Attack)), (1, 5, 5, 4));
    }

    #[test]
    fn real_clubs_get_full_valid_selections_that_rate_no_higher_than_their_best_eleven() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../history.db");
        if !std::path::Path::new(path).exists() { return; } // needs `cargo run -p import` first
        for edition in [15, 24] {
            let w = crate::source::load_edition(&rusqlite::Connection::open(path).unwrap(), edition).unwrap();
            let (mut full, mut keeper_in_goal, mut clubs) = (0, 0, 0);
            for c in w.clubs.values().filter(|c| c.squad.len() >= 18) {
                clubs += 1;
                let selection = w.selection_of(c.id);
                full += (selection.lineup.iter().flatten().count() == 11 && selection.bench.iter().flatten().count() == BENCH_SIZE) as usize;
                keeper_in_goal += selection.lineup[0].is_some_and(|id| w.players[&id].positions.contains(&Gk)) as usize;

                // The best eleven by overall is an upper bound, give or take what a player's attributes add elsewhere.
                let mut best: Vec<u8> = c.squad.iter().map(|p| w.players[p].overall).collect();
                best.sort_by(|a, b| b.cmp(a));
                let best_eleven: u32 = best.iter().take(11).map(|&o| o as u32).sum();
                let lineup_total: u32 = w.lineup_ratings(c.id).iter().map(|&r| r as u32).sum();
                assert!(lineup_total <= best_eleven + 5, "{}: lineup {lineup_total} beats the best eleven {best_eleven}", c.name);
            }
            assert!(clubs > 300, "edition {edition}: only {clubs} clubs");
            assert_eq!(full, clubs, "edition {edition}: every club with a real squad fills eleven slots and the bench");
            assert!(keeper_in_goal * 100 >= clubs * 99, "edition {edition}: only {keeper_in_goal} of {clubs} have a keeper in goal");
        }
    }

    /// A squad of a keeper, 4 defenders, 3 midfielders and three forwards, every player rated 70.
    fn squad_with_three_forwards() -> Vec<Player> {
        let positions = [Gk, Lb, Cb, Cb, Rb, Cm, Cm, Cm, Lw, St, Rw];
        positions.iter().enumerate().map(|(i, &p)| player(i as u32 + 1, 70, &[p])).collect()
    }

    #[test]
    fn a_squad_built_for_three_forwards_gets_a_formation_with_three_forwards() {
        let squad = squad_with_three_forwards();
        let refs: Vec<&Player> = squad.iter().collect();
        let formation = best_formation(&refs);
        let forwards = formation.slots().iter().filter(|p| p.line() == Line::Attack).count();
        assert_eq!(forwards, 3, "{}", formation.name());
    }

    #[test]
    fn the_chosen_formation_never_rates_below_any_other() {
        let squad = squad_with_three_forwards();
        let refs: Vec<&Player> = squad.iter().collect();
        let total = |f: Formation| -> u32 {
            let lineup = pick_lineup(&refs, f);
            f.slots().iter().zip(lineup).map(|(&slot, id)| rating_at(refs.iter().find(|p| Some(p.id) == id).unwrap(), slot) as u32).sum()
        };
        let best = total(best_formation(&refs));
        assert!(Formation::ALL.iter().all(|&f| total(f) <= best));
    }

    #[test]
    fn a_tie_keeps_the_first_formation_and_a_formation_the_user_set_is_left_alone() {
        let mut w = World::new(0, 0);
        crate::testing::add_club_with_eleven(&mut w, 1, 1, 70); // natural 4-4-2 players: nothing beats 4-4-2
        crate::testing::add_club_with_eleven(&mut w, 2, 1, 70);
        w.clubs.get_mut(&2).unwrap().formation = Formation::ThreeFiveTwo;
        w.clubs.get_mut(&2).unwrap().formation_set = true;
        w.choose_formations();
        assert_eq!(w.clubs[&1].formation, Formation::FourFourTwo);
        assert_eq!(w.clubs[&2].formation, Formation::ThreeFiveTwo);
    }

    #[test]
    fn a_centre_forward_and_a_striker_are_the_same_role() {
        let cf_only = player_with_attrs(1, 90, &[Cf], &[(Attr::Finishing, 90), (Attr::Heading, 30), (Attr::Strength, 30)]);
        assert_eq!(rating_at(&cf_only, St), 90, "no penalty and no formula gap");
        assert_eq!(familiarity(&cf_only, St), Familiarity::Natural);
        let st_only = player(2, 85, &[St]);
        assert_eq!(rating_at(&st_only, Cf), 85);
        assert_ne!(familiarity(&cf_only, Lw), Familiarity::Natural, "a winger is still another role");
    }
}
