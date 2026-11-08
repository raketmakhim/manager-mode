//! The lineup view: the starting eleven on a pitch, then the substitutes and the reserves. Click a player and then
//! someone else to swap them.
use crate::{GREEN, RED};
use eframe::egui::{self, Color32, RichText, Vec2};
use engine::{
    familiarity, rating_at, team_strength, ClubId, Familiarity, Formation, Player, Selection, Spot, World,
    BENCH_SIZE,
};

const AMBER: Color32 = Color32::from_rgb(230, 170, 50);
const CARD: Vec2 = Vec2::new(118.0, 58.0);
const SPACING: f32 = 6.0;
/// The most players in one row of any formation, which sets the width of the pitch.
const WIDEST_ROW: f32 = 5.0;

/// What the view remembers between frames.
#[derive(Default)]
pub struct State {
    /// The player waiting to be swapped with whoever is clicked next.
    selected: Option<Spot>,
}

/// Shows and edits the lineup and bench of `club`.
pub fn ui(ui: &mut egui::Ui, world: &mut World, club: ClubId, state: &mut State) {
    // Only what the user changes is stored. Until then the club stays automatic and the game re-picks before each
    // match, so the lineup keeps up with new ratings.
    let current = world.selection_of(club);

    let mut formation = world.clubs[&club].formation;
    let mut pick_best = false;
    ui.horizontal(|ui| {
        ui.label("Formation:");
        egui::ComboBox::from_id_salt("formation").selected_text(formation.name()).show_ui(ui, |ui| {
            for &f in Formation::ALL {
                ui.selectable_value(&mut formation, f, f.name());
            }
        });
        pick_best = ui.button("Automatic: pick the best eleven").clicked();
        ui.label(format!("Team strength {:.1}", team_strength(&world.lineup_ratings(club))));
    });
    if formation != world.clubs[&club].formation || pick_best {
        // A different formation has different slots, so a set selection no longer fits; the selection goes back to
        // automatic. Choosing a formation keeps it; the Automatic button lets the game choose it again.
        let c = world.clubs.get_mut(&club).unwrap();
        c.formation = formation;
        c.formation_set = !pick_best;
        c.selection = Selection::default();
        state.selected = None;
        if pick_best {
            world.choose_formation(club);
        }
    }
    ui.label(if world.has_set_selection(club) {
        "Your own selection: the game keeps it until you press Automatic."
    } else {
        "Automatic: the game picks the best eleven before each match. Swap anyone to take over."
    });
    ui.label("Click a player, then click someone else to swap them. Click the same player again to cancel.");
    ui.add_space(6.0);

    let clicked = {
        let world: &World = world;
        let selection = current;
        let (mut on_pitch, mut in_lists) = (None, None);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(WIDEST_ROW * (CARD.x + SPACING));
                on_pitch = pitch(ui, world, club, &selection, state);
            });
            ui.add_space(12.0);
            ui.vertical(|ui| in_lists = side_lists(ui, world, club, &selection, state));
        });
        on_pitch.or(in_lists)
    };
    if let Some(spot) = clicked {
        match state.selected {
            Some(first) if first == spot => state.selected = None,
            Some(first) => {
                let mut changed = current;
                changed.swap(first, spot);
                let c = world.clubs.get_mut(&club).unwrap();
                c.selection = changed;
                c.formation_set = true; // the lineup was made for this formation, so keep it
                state.selected = None;
            }
            None => state.selected = Some(spot),
        }
    }
}

/// The player selected for a swap, if he is not a starter: his rating in each starting slot is shown on the pitch.
fn waiting_player<'a>(world: &'a World, selection: &Selection, state: &State) -> Option<&'a Player> {
    match state.selected? {
        Spot::Starter(_) => None,
        spot => selection.player_at(spot).map(|id| &world.players[&id]),
    }
}

/// The starting eleven in formation, attack at the top. Returns the spot clicked, if any.
fn pitch(ui: &mut egui::Ui, world: &World, club: ClubId, selection: &Selection, state: &State) -> Option<Spot> {
    let rows = world.clubs[&club].formation.rows();
    let waiting = waiting_player(world, selection, state);
    let mut clicked = None;
    // A row's slots are numbered after the rows below it; draw from the attack down to the goalkeeper.
    let first_slot: Vec<usize> = rows.iter().scan(0, |next, row| Some(std::mem::replace(next, *next + row.len()))).collect();
    for (row, first) in rows.iter().zip(first_slot).rev() {
        let row_width = row.len() as f32 * (CARD.x + SPACING) - SPACING;
        ui.horizontal(|ui| {
            ui.add_space(((ui.available_width() - row_width) / 2.0).max(0.0));
            for (i, &position) in row.iter().enumerate() {
                let slot = first + i;
                let player = selection.lineup[slot].map(|id| &world.players[&id]);
                let (text, colour) = match player {
                    None => (format!("{}\n(empty)", position.code()), Color32::GRAY),
                    Some(p) => {
                        let mut rating = format!("{}", rating_at(p, position));
                        if let Some(w) = waiting {
                            rating += &format!("   -> {}", rating_at(w, position));
                        }
                        (format!("{}\n{}\n{}", position.code(), short_name(&p.name), rating), familiarity_colour(familiarity(p, position)))
                    }
                };
                let selected = state.selected == Some(Spot::Starter(slot));
                if ui.add(egui::Button::new(RichText::new(text).color(colour)).min_size(CARD).selected(selected)).clicked() {
                    clicked = Some(Spot::Starter(slot));
                }
            }
        });
        ui.add_space(SPACING);
    }
    clicked
}

/// The substitutes and the reserves. When a starter is selected, each shows the rating he would have in that slot.
fn side_lists(ui: &mut egui::Ui, world: &World, club: ClubId, selection: &Selection, state: &State) -> Option<Spot> {
    let slots = world.clubs[&club].formation.slots();
    let target = match state.selected {
        Some(Spot::Starter(slot)) => Some(slots[slot]),
        _ => None,
    };
    let mut clicked = None;
    let mut row = |ui: &mut egui::Ui, spot: Spot, player: Option<&Player>| {
        let (text, colour) = match player {
            None => ("(empty)".to_string(), Color32::GRAY),
            Some(p) => {
                let mut text = format!("{:3}  {}  ({})", p.overall, p.name, positions_text(p));
                let mut colour = ui.visuals().text_color();
                if let Some(position) = target {
                    text += &format!("   -> {} as {}", rating_at(p, position), position.code());
                    colour = familiarity_colour(familiarity(p, position));
                }
                (text, colour)
            }
        };
        let selected = state.selected == Some(spot);
        if ui.add(egui::Button::new(RichText::new(text).color(colour)).selected(selected)).clicked() {
            clicked = Some(spot);
        }
    };

    ui.label(RichText::new(format!("Substitutes ({BENCH_SIZE})")).strong());
    for place in 0..BENCH_SIZE {
        let player = selection.bench[place].map(|id| &world.players[&id]);
        row(ui, Spot::Sub(place), player);
    }
    ui.add_space(10.0);

    let mut reserves: Vec<&Player> = world.clubs[&club]
        .squad
        .iter()
        .filter(|id| !selection.lineup.contains(&Some(**id)) && !selection.bench.contains(&Some(**id)))
        .map(|id| &world.players[id])
        .collect();
    reserves.sort_by_key(|p| std::cmp::Reverse(p.overall));
    ui.label(RichText::new(format!("Reserves ({})", reserves.len())).strong());
    egui::ScrollArea::vertical().id_salt("reserves").show(ui, |ui| {
        for p in reserves {
            row(ui, Spot::Reserve(p.id), Some(p));
        }
    });
    clicked
}

/// Green for a listed position, amber for the same line, red for anywhere else.
fn familiarity_colour(familiarity: Familiarity) -> Color32 {
    match familiarity {
        Familiarity::Natural => GREEN,
        Familiarity::SameLine => AMBER,
        Familiarity::OtherLine => RED,
    }
}

/// "ST, LW".
fn positions_text(p: &Player) -> String {
    p.positions.iter().map(|pos| pos.code()).collect::<Vec<_>>().join(", ")
}

/// "Marcus Rashford" becomes "M. Rashford"; a single name stays as it is.
pub(crate) fn short_name(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    match words.as_slice() {
        [] => String::new(),
        [only] => only.to_string(),
        [first, .., last] => format!("{}. {last}", first.chars().next().unwrap_or(' ')),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_shortened_to_an_initial_and_the_last_word() {
        assert_eq!(short_name("Marcus Rashford"), "M. Rashford");
        assert_eq!(short_name("Wayne Mark Rooney"), "W. Rooney");
        assert_eq!(short_name("Pele"), "Pele");
        assert_eq!(short_name(""), "");
    }
}
