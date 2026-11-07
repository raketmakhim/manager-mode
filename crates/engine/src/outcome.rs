//! The result of a played match.
use std::cmp::Ordering;
use std::fmt;

/// The home or the away team of a match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Home,
    Away,
}

/// What happened in a match. A league match only has `full_time`; a cup match can go to extra time
/// and then a penalty shootout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// Goals after 90 minutes: (home, away).
    pub full_time: (u8, u8),
    /// Goals scored in extra time only, if extra time was played.
    pub extra_time: Option<(u8, u8)>,
    /// Penalty shootout score, if there was one. Shootout goals never count as scorer stats.
    pub shootout: Option<(u8, u8)>,
}

impl Outcome {
    /// A match decided in 90 minutes.
    pub fn full_time(home: u8, away: u8) -> Outcome {
        Outcome { full_time: (home, away), extra_time: None, shootout: None }
    }

    /// Goals over the whole match: 90 minutes plus extra time, shootout not included.
    pub fn goals(&self) -> (u8, u8) {
        let (home, away) = self.extra_time.unwrap_or((0, 0));
        (self.full_time.0 + home, self.full_time.1 + away)
    }

    /// Who won, or `None` for a draw (a league match, or a first leg).
    pub fn winner(&self) -> Option<Side> {
        let (home, away) = self.goals();
        let (home, away) = if home == away { self.shootout.unwrap_or((0, 0)) } else { (home, away) };
        match home.cmp(&away) {
            Ordering::Greater => Some(Side::Home),
            Ordering::Less => Some(Side::Away),
            Ordering::Equal => None,
        }
    }
}

/// "2-1", "2-1 aet" after extra time, "2-2 aet (4-3 pens)" after a shootout.
impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (home, away) = self.goals();
        write!(f, "{home}-{away}")?;
        if self.extra_time.is_some() {
            write!(f, " aet")?;
        }
        if let Some((home, away)) = self.shootout {
            write!(f, " ({home}-{away} pens)")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extra_time(full_time: (u8, u8), extra: (u8, u8)) -> Outcome {
        Outcome { full_time, extra_time: Some(extra), shootout: None }
    }

    fn shootout(full_time: (u8, u8), extra: Option<(u8, u8)>, pens: (u8, u8)) -> Outcome {
        Outcome { full_time, extra_time: extra, shootout: Some(pens) }
    }

    #[test]
    fn a_90_minute_match_has_a_winner_or_is_a_draw() {
        assert_eq!(Outcome::full_time(2, 1).winner(), Some(Side::Home));
        assert_eq!(Outcome::full_time(0, 3).winner(), Some(Side::Away));
        assert_eq!(Outcome::full_time(1, 1).winner(), None);
    }

    #[test]
    fn extra_time_goals_count_towards_the_score_and_the_winner() {
        let o = extra_time((1, 1), (0, 1));
        assert_eq!(o.goals(), (1, 2));
        assert_eq!(o.winner(), Some(Side::Away));
    }

    #[test]
    fn a_shootout_decides_a_tie_but_its_goals_are_not_match_goals() {
        let o = shootout((2, 2), Some((0, 0)), (3, 4));
        assert_eq!(o.goals(), (2, 2));
        assert_eq!(o.winner(), Some(Side::Away));
        // Straight to penalties, as in a cup round with no extra time.
        assert_eq!(shootout((0, 0), None, (5, 4)).winner(), Some(Side::Home));
    }

    #[test]
    fn the_match_score_beats_the_shootout_when_there_is_one() {
        // A shootout score recorded after a decisive match is ignored.
        assert_eq!(shootout((2, 1), None, (0, 5)).winner(), Some(Side::Home));
    }

    #[test]
    fn it_prints_the_way_a_scoreboard_would() {
        assert_eq!(Outcome::full_time(2, 1).to_string(), "2-1");
        assert_eq!(extra_time((1, 1), (1, 0)).to_string(), "2-1 aet");
        assert_eq!(shootout((2, 2), Some((0, 0)), (4, 3)).to_string(), "2-2 aet (4-3 pens)");
        assert_eq!(shootout((1, 1), None, (4, 3)).to_string(), "1-1 (4-3 pens)");
    }
}
