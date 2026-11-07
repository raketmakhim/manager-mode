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
    fn every_attr_is_in_exactly_one_list() {
        for &a in Attr::ALL {
            let n = OUTFIELD.iter().chain(KEEPER).filter(|&&x| x == a).count();
            assert_eq!(n, 1, "{a:?} appears {n} times across OUTFIELD and KEEPER");
        }
        assert_eq!(OUTFIELD.len() + KEEPER.len(), COUNT);
    }
}
