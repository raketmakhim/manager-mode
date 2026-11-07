//! Player attributes: the `Attr` names, which ones outfield players and keepers have, and `Attrs` to hold a rating for each.
use std::ops::Index;

/// Defines `Attr` plus its on-screen label and its `ratings` column in one place, so the three cannot drift apart.
/// Each line is: variant name, label, database column.
macro_rules! attrs {
    ($($name:ident $label:literal $col:literal),* $(,)?) => {
        /// One player attribute, e.g. `Attr::Finishing`.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Attr { $($name),* }

        impl Attr {
            /// Every attribute, in the order they are listed below.
            pub const ALL: &'static [Attr] = &[$(Attr::$name),*];
            /// Name shown on screen, e.g. "Short passing".
            pub fn label(self) -> &'static str { match self { $(Attr::$name => $label),* } }
            /// Group it belongs to, the start of its column name: "attacking", "skill", "goalkeeping"...
            pub fn category(self) -> &'static str { self.column().split('_').next().unwrap_or("") }
            /// Column in the `ratings` table, e.g. "attacking_short_passing".
            pub fn column(self) -> &'static str { match self { $(Attr::$name => $col),* } }
        }
    };
}

// Categories A-Z, attributes A-Z inside each. The category is the start of the column name.
attrs! {
    Crossing        "Crossing"        "attacking_crossing",
    Finishing       "Finishing"       "attacking_finishing",
    Heading         "Heading"         "attacking_heading_accuracy",
    ShortPassing    "Short passing"   "attacking_short_passing",
    Volleys         "Volleys"         "attacking_volleys",

    Marking         "Marking"         "defending_marking_awareness",
    SlidingTackle   "Sliding tackle"  "defending_sliding_tackle",
    StandingTackle  "Standing tackle" "defending_standing_tackle",

    GkDiving        "GK diving"       "goalkeeping_diving",
    GkHandling      "GK handling"     "goalkeeping_handling",
    GkKicking       "GK kicking"      "goalkeeping_kicking",
    GkPositioning   "GK positioning"  "goalkeeping_positioning",
    GkReflexes      "GK reflexes"     "goalkeeping_reflexes",

    Aggression      "Aggression"      "mentality_aggression",
    Composure       "Composure"       "mentality_composure",
    Interceptions   "Interceptions"   "mentality_interceptions",
    Penalties       "Penalties"       "mentality_penalties",
    Positioning     "Positioning"     "mentality_positioning",
    Vision          "Vision"          "mentality_vision",

    Acceleration    "Acceleration"    "movement_acceleration",
    Agility         "Agility"         "movement_agility",
    Balance         "Balance"         "movement_balance",
    Reactions       "Reactions"       "movement_reactions",
    SprintSpeed     "Sprint speed"    "movement_sprint_speed",

    Jumping         "Jumping"         "power_jumping",
    LongShots       "Long shots"      "power_long_shots",
    ShotPower       "Shot power"      "power_shot_power",
    Stamina         "Stamina"         "power_stamina",
    Strength        "Strength"        "power_strength",

    BallControl     "Ball control"    "skill_ball_control",
    Curve           "Curve"           "skill_curve",
    Dribbling       "Dribbling"       "skill_dribbling",
    FkAccuracy      "FK accuracy"     "skill_fk_accuracy",
    LongPassing     "Long passing"    "skill_long_passing",
}

use Attr::*;

/// The attributes shown for every player except goalkeepers, in on-screen order (same order as above).
pub const OUTFIELD: &[Attr] = &[
    Crossing,
    Finishing,
    Heading,
    ShortPassing,
    Volleys,

    Marking,
    SlidingTackle,
    StandingTackle,

    Aggression,
    Composure,
    Interceptions,
    Penalties,
    Positioning,
    Vision,

    Acceleration,
    Agility,
    Balance,
    Reactions,
    SprintSpeed,

    Jumping,
    LongShots,
    ShotPower,
    Stamina,
    Strength,

    BallControl,
    Curve,
    Dribbling,
    FkAccuracy,
    LongPassing,
];

/// The attributes shown for goalkeepers.
pub const KEEPER: &[Attr] = &[
    GkDiving,
    GkHandling,
    GkKicking,
    GkPositioning,
    GkReflexes,
];

const COUNT: usize = Attr::ALL.len();

/// One rating (0-99) per attribute, read as `attrs[Attr::Finishing]`.
/// `None` means that edition has no value for it (e.g. composure in FIFA 15).
#[derive(Clone, Copy, Debug)]
pub struct Attrs([Option<u8>; COUNT]);

impl Attr {
    /// `r.attacking_crossing, r.attacking_finishing, ...` for a query that reads `ratings AS r`.
    pub(crate) fn select_list() -> String {
        let cols: Vec<String> = Attr::ALL.iter().map(|a| format!("r.{}", a.column())).collect();
        cols.join(", ")
    }
}

impl Attrs {
    /// Reads every attribute from a row of a query that selected `Attr::select_list()`.
    pub(crate) fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Attrs> {
        let mut attrs = Attrs::default();
        for &a in Attr::ALL {
            attrs.0[a as usize] = row.get(a.column())?;
        }
        Ok(attrs)
    }
}

/// A rating after the real game changed it from `before` to `after`: the current rating moves by the same amount,
/// kept inside 0-99. A player nobody touched ends exactly on the real `after`; one whose rating drifted (trained
/// up, left on the bench) keeps the difference. This is how the player can make or break someone.
pub fn follow_real_change(current: u8, before: u8, after: u8) -> u8 {
    (current as i16 + after as i16 - before as i16).clamp(0, 99) as u8
}

impl Attrs {
    /// Every attribute follows the real change from `before` to `after` (see `follow_real_change`). An attribute
    /// the old edition did not have takes the new real value; one the new edition lacks stays as it is.
    pub(crate) fn follow_real_change(&self, before: &Attrs, after: &Attrs) -> Attrs {
        let mut next = *self;
        for &a in Attr::ALL {
            next.0[a as usize] = match (self[a], before[a], after[a]) {
                (Some(now), Some(was), Some(will_be)) => Some(follow_real_change(now, was, will_be)),
                (_, _, Some(will_be)) => Some(will_be),
                (now, _, None) => now,
            };
        }
        next
    }
}


impl Default for Attrs {
    fn default() -> Self { Attrs([None; COUNT]) } // arrays this long have no derived Default
}

impl Index<Attr> for Attrs {
    type Output = Option<u8>;
    fn index(&self, a: Attr) -> &Option<u8> { &self.0[a as usize] }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_list_keeps_a_category_together() {
        for list in [OUTFIELD, KEEPER] {
            let groups: Vec<&str> = list.chunk_by(|a, b| a.category() == b.category()).map(|g| g[0].category()).collect();
            let mut sorted = groups.clone();
            sorted.sort();
            sorted.dedup();
            assert_eq!(groups.len(), sorted.len(), "a category is split in two: {groups:?}");
        }
    }

    #[test]
    fn an_attribute_knows_its_category() {
        assert_eq!(Attr::Finishing.category(), "attacking");
        assert_eq!(Attr::GkDiving.category(), "goalkeeping");
        assert_eq!(Attr::StandingTackle.category(), "defending");
    }

    #[test]
    fn every_attr_is_in_exactly_one_list() {
        for &a in Attr::ALL {
            let n = OUTFIELD.iter().chain(KEEPER).filter(|&&x| x == a).count();
            assert_eq!(n, 1, "{a:?} appears {n} times across OUTFIELD and KEEPER");
        }
        assert_eq!(OUTFIELD.len() + KEEPER.len(), COUNT);
    }

    #[test]
    fn a_rating_follows_the_real_change_and_keeps_its_drift() {
        assert_eq!(follow_real_change(70, 70, 75), 75, "untouched player lands on the real rating");
        assert_eq!(follow_real_change(78, 70, 75), 83, "a player trained up by 8 keeps the 8");
        assert_eq!(follow_real_change(65, 70, 75), 70, "one left to decline keeps the gap");
        assert_eq!(follow_real_change(97, 90, 95), 99, "never above 99");
        assert_eq!(follow_real_change(3, 40, 30), 0, "never below 0");
    }

    #[test]
    fn attributes_follow_the_real_change_and_fill_in_new_ones() {
        let mut before = Attrs::default();
        let mut after = Attrs::default();
        let mut now = Attrs::default();
        before.0[Attr::Finishing as usize] = Some(70);
        after.0[Attr::Finishing as usize] = Some(74);
        now.0[Attr::Finishing as usize] = Some(72);
        after.0[Attr::Composure as usize] = Some(60); // not in the old edition
        now.0[Attr::Heading as usize] = Some(80); // not in the new edition
        let next = now.follow_real_change(&before, &after);
        assert_eq!(next[Attr::Finishing], Some(76));
        assert_eq!(next[Attr::Composure], Some(60));
        assert_eq!(next[Attr::Heading], Some(80));
    }
}
