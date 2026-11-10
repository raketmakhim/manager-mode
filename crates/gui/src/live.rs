//! The live match window: your match played a minute at a time. The clock runs at a speed you pick, stops by itself at
//! half time and full time (and at goals if you ask), and you can make changes from the bench, swap players and change the formation at any time.
use crate::lineup::short_name;
use crate::{GREEN, RED};
use eframe::egui::{self, Color32, ProgressBar, RichText, Vec2};
use engine::{live_match_stats, Event, Formation, Line, LiveMatch, MatchTeam, PlayerId, PlayerStats, Rng, Side, SubstitutionError, TacticsError, World};
use std::time::Duration;

/// How long a minute of the match lasts for each speed, in real seconds, and what to call it.
const SPEEDS: [(&str, f32); 9] = [
    ("Real time (60 s a minute)", 60.0),
    ("30 s a minute", 30.0),
    ("15 s a minute", 15.0),
    ("10 s a minute", 10.0),
    ("5 s a minute", 5.0),
    ("2 s a minute", 2.0),
    ("1 s a minute", 1.0),
    ("Whole match in 45 s", 0.5),
    ("Whole match in 10 s", 10.0 / 90.0),
];
/// The speed picked when a match opens: an index into `SPEEDS`.
const DEFAULT_SPEED: usize = 3;
/// The clock stops by itself at half time, at the end of 90 minutes (if extra time follows) and at its break.
const BREAKS: [u8; 3] = [45, 90, 105];
/// The height of the match feed, in points.
const FEED_HEIGHT: f32 = 330.0;
/// The width of the column with the feed and the stats, in points; the rest is the pitch.
const SIDE_WIDTH: f32 = 330.0;
/// The size of a player's card on the pitch and the bench.
const CARD: Vec2 = Vec2::new(112.0, 50.0);
/// The fill of the card of the player picked.
const PICKED: Color32 = Color32::from_rgb(60, 110, 180);

/// A match being watched.
pub struct State {
    fixture: usize,
    live: LiveMatch,
    /// Your side.
    side: Side,
    /// The formation your side is set up in now.
    formation: Formation,
    playing: bool,
    speed: usize,
    pause_on_goals: bool,
    /// Real seconds since the last minute was played.
    clock: f32,
    /// The player clicked first: the next click on another player swaps their places, or on a substitute brings him on.
    picked: Option<PlayerId>,
    /// Why the last change was refused.
    message: Option<String>,
    /// What each player earned in the match, worked out once at full time.
    result: Option<Vec<(PlayerId, PlayerStats)>>,
}

impl State {
    pub fn new(fixture: usize, live: LiveMatch, side: Side, formation: Formation) -> State {
        State { fixture, live, side, formation, playing: false, speed: DEFAULT_SPEED, pause_on_goals: true, clock: 0.0, picked: None, message: None, result: None }
    }

    /// The fixture and the match, to hand back to the world when the match is done with.
    pub fn into_parts(self) -> (usize, LiveMatch) {
        (self.fixture, self.live)
    }
}

/// Shows the match and runs its clock. True when you press Continue after full time.
pub fn ui(ui: &mut egui::Ui, world: &World, s: &mut State, rng: &mut Rng) -> bool {
    run_clock(ui, s, rng);
    let f = &world.fixtures[s.fixture];
    let (home, away) = (&world.clubs[&f.home].name, &world.clubs[&f.away].name);
    let (home_goals, away_goals) = s.live.score();
    ui.horizontal(|ui| {
        ui.heading(format!("{home}  {home_goals} - {away_goals}  {away}"));
        ui.label(RichText::new(clock_text(&s.live)).heading());
    });
    let done = controls_ui(ui, s, rng);
    ui.separator();
    egui::ScrollArea::vertical().id_salt("live_screen").show(ui, |ui| {
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(SIDE_WIDTH);
                feed_ui(ui, world, s);
                ui.add_space(8.0);
                stats_ui(ui, home, away, &s.live);
            });
            ui.separator();
            ui.vertical(|ui| {
                team_ui(ui, world, s);
                if s.live.is_over() {
                    ui.add_space(8.0);
                    xp_ui(ui, world, s);
                }
            });
        });
    });
    done
}

/// Plays the minutes that the real time passed since the last frame is worth.
fn run_clock(ui: &egui::Ui, s: &mut State, rng: &mut Rng) {
    if !s.playing || s.live.is_over() {
        return;
    }
    s.clock += ui.ctx().input(|i| i.stable_dt);
    let minute_length = SPEEDS[s.speed].1;
    while s.playing && s.clock >= minute_length && !s.live.is_over() {
        s.clock -= minute_length;
        play_minute(s, rng);
    }
    ui.ctx().request_repaint_after(Duration::from_millis(40));
}

/// Plays one minute and stops the clock if it is a break, full time, or a goal and you asked to stop at goals.
fn play_minute(s: &mut State, rng: &mut Rng) {
    let seen = s.live.events().len();
    s.live.advance(rng);
    let goal = s.live.events()[seen..].iter().any(|e| matches!(e, Event::Goal { .. }));
    if s.live.is_over() || BREAKS.contains(&s.live.minute()) || (goal && s.pause_on_goals) {
        s.playing = false;
        s.clock = 0.0;
    }
}

fn clock_text(live: &LiveMatch) -> String {
    match live.minute() {
        _ if live.is_over() => "Full time".to_string(),
        0 => "Kick-off".to_string(),
        45 => "Half time".to_string(),
        90 => "Full time: extra time next".to_string(),
        m => format!("{m}'"),
    }
}

/// Play and pause, speed, and the way out. True when Continue is pressed.
fn controls_ui(ui: &mut egui::Ui, s: &mut State, rng: &mut Rng) -> bool {
    let mut done = false;
    ui.horizontal(|ui| {
        if s.live.is_over() {
            done = ui.button("Continue").clicked();
            return;
        }
        let label = if s.playing { "Pause" } else if s.live.minute() == 0 { "Kick off" } else { "Play" };
        if ui.button(label).clicked() {
            s.playing = !s.playing;
            s.clock = 0.0;
        }
        egui::ComboBox::from_id_salt("live_speed").selected_text(SPEEDS[s.speed].0).show_ui(ui, |ui| {
            for (i, (name, _)) in SPEEDS.iter().enumerate() {
                ui.selectable_value(&mut s.speed, i, *name);
            }
        });
        ui.checkbox(&mut s.pause_on_goals, "Stop at goals");
        if ui.button("Skip to full time").clicked() {
            // Your side stops being run by hand: the game makes the changes you have left.
            s.live.hand_over(s.side, rng);
            while !s.live.is_over() {
                s.live.advance(rng);
            }
            s.playing = false;
        }
    });
    done
}

/// What happened, newest first.
fn feed_ui(ui: &mut egui::Ui, world: &World, s: &State) {
    ui.heading("Match");
    egui::ScrollArea::vertical().id_salt("live_feed").max_height(FEED_HEIGHT).show(ui, |ui| {
        for event in s.live.events().iter().rev() {
            if let Some((minute, text, colour)) = describe(world, event, s) {
                let line = RichText::new(format!("{minute}'  {text}"));
                ui.label(colour.map_or(line.clone(), |c| line.color(c)));
            }
        }
        if s.live.events().is_empty() {
            ui.label("Waiting for kick-off.");
        }
    });
}

/// The minute, a line of text and its colour (`None` for ordinary text) for the events worth showing; `None` for the rest.
fn describe(world: &World, event: &Event, s: &State) -> Option<(u8, String, Option<Color32>)> {
    let name = |id: PlayerId| world.players.get(&id).map_or("?".to_string(), |p| p.name.clone());
    let club = |side: Side| {
        let f = &world.fixtures[s.fixture];
        world.clubs[&if side == Side::Home { f.home } else { f.away }].name.clone()
    };
    Some(match *event {
        Event::Goal { minute, side, scorer, assister } => {
            let assist = assister.map_or(String::new(), |a| format!(" (assist {})", name(a)));
            (minute, format!("GOAL for {}: {}{assist}", club(side), name(scorer)), Some(GREEN))
        }
        Event::Shot { minute, shooter, on_target: false, .. } => (minute, format!("{} shoots wide", name(shooter)), None),
        Event::Save { minute, keeper, .. } => (minute, format!("Save by {}", name(keeper)), None),
        Event::Corner { minute, side } => (minute, format!("Corner for {}", club(side)), None),
        Event::Yellow { minute, player, .. } => (minute, format!("Yellow card: {}", name(player)), Some(Color32::from_rgb(230, 190, 40))),
        Event::Red { minute, player, .. } => (minute, format!("RED CARD: {}", name(player)), Some(RED)),
        Event::Substitution { minute, side, off, on, .. } => (minute, format!("Change for {}: {} on, {} off", club(side), name(on), name(off)), None),
        _ => return None,
    })
}

/// Your eleven on a pitch (attack at the top) with how fresh each is, your bench, and your tactics.
fn team_ui(ui: &mut egui::Ui, world: &World, s: &mut State) {
    let team = *s.live.team(s.side);
    let tiredness = s.live.tiredness(s.side);
    let name = |id: PlayerId| world.players.get(&id).map_or("?".to_string(), |p| short_name(&p.name));
    let mut clicked = None;
    ui.heading("Your team");
    ui.label(format!("Changes left: {}. Click a player, then another player to swap their places, or a substitute to bring him on.", s.live.substitutions_left(s.side)));
    for line in [Line::Attack, Line::Midfield, Line::Defence, Line::Goalkeeper] {
        ui.horizontal(|ui| {
            for (slot, p) in team.players.iter().enumerate().filter(|(_, p)| p.position.line() == line) {
                ui.vertical(|ui| {
                    if p.is_empty() {
                        ui.add_enabled(false, egui::Button::new(format!("{}\nsent off", p.position.code())).min_size(CARD));
                        return;
                    }
                    if card(ui, s.picked == Some(p.id), format!("{}  {}\n{}", p.position.code(), p.rating, name(p.id))) {
                        clicked = Some(p.id);
                    }
                    let energy = 1.0 - tiredness[slot];
                    ui.add(ProgressBar::new(energy as f32).desired_width(CARD.x).text(format!("{:.0}%", 100.0 * energy)));
                });
            }
        });
        ui.add_space(4.0);
    }
    ui.label(RichText::new("Bench").strong());
    ui.horizontal_wrapped(|ui| {
        for p in team.bench.iter().filter(|p| !p.is_empty()) {
            if card(ui, s.picked == Some(p.id), format!("{}  {}\n{}", p.position.code(), p.rating, name(p.id))) {
                clicked = Some(p.id);
            }
        }
    });
    if let Some(id) = clicked {
        click(s, id, &team);
    }
    ui.add_space(6.0);
    tactics_ui(ui, s);
    if let Some(message) = &s.message {
        ui.colored_label(RED, message);
    }
}

/// At full time: what each of your players did and the XP he earned.
fn xp_ui(ui: &mut egui::Ui, world: &World, s: &mut State) {
    let f = &world.fixtures[s.fixture];
    let club = if s.side == Side::Home { f.home } else { f.away };
    let stats = s.result.get_or_insert_with(|| live_match_stats(&s.live));
    ui.heading("Match XP");
    egui::Grid::new("live_xp").striped(true).show(ui, |ui| {
        for h in ["Player", "Min", "Rating", "XP"] {
            ui.label(RichText::new(h).strong());
        }
        ui.end_row();
        for (id, st) in stats.iter().filter(|(id, _)| world.players.get(id).is_some_and(|p| p.club == Some(club))) {
            ui.label(&world.players[id].name);
            ui.label(st.minutes.to_string());
            ui.label(st.average_rating().map_or("-".to_string(), |r| format!("{r:.1}")));
            ui.label(RichText::new(st.xp.to_string()).strong());
            ui.end_row();
        }
    });
}

/// A player's card; true when it is clicked.
fn card(ui: &mut egui::Ui, picked: bool, text: String) -> bool {
    let fill = if picked { PICKED } else { ui.visuals().widgets.inactive.weak_bg_fill };
    ui.add(egui::Button::new(text).fill(fill).min_size(CARD)).clicked()
}

/// A click on a player. The first click picks him; the second, on someone else, does the change: two players on the
/// pitch trade places, a player on the pitch and a substitute trade (a change), two substitutes just move the pick.
fn click(s: &mut State, id: PlayerId, team: &MatchTeam) {
    let Some(first) = s.picked.filter(|&first| first != id) else {
        s.picked = if s.picked == Some(id) { None } else { Some(id) };
        return;
    };
    let on_pitch = |id| team.players.iter().any(|p| p.id == id && !p.is_empty());
    s.picked = None;
    s.message = match (on_pitch(first), on_pitch(id)) {
        (true, true) => s.live.swap_positions(s.side, first, id).err().map(tactics_refusal),
        (true, false) => s.live.substitute(s.side, first, id).err().map(|e| refusal(e).to_string()),
        (false, true) => s.live.substitute(s.side, id, first).err().map(|e| refusal(e).to_string()),
        (false, false) => {
            s.picked = Some(id);
            None
        }
    };
}

/// The formation, which takes effect from the next minute.
fn tactics_ui(ui: &mut egui::Ui, s: &mut State) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Formation:").strong());
        let before = s.formation;
        egui::ComboBox::from_id_salt("live_formation").selected_text(s.formation.name()).show_ui(ui, |ui| {
            for &f in Formation::ALL {
                ui.selectable_value(&mut s.formation, f, f.name());
            }
        });
        if s.formation != before {
            s.live.set_formation(s.side, s.formation.slots());
            s.picked = None;
        }
    });
}

fn tactics_refusal(error: TacticsError) -> String {
    match error {
        TacticsError::NotOnPitch => "That player is not on the pitch.",
        TacticsError::Keeper => "A goalkeeper cannot swap places with an outfield player.",
    }
    .to_string()
}

fn refusal(error: SubstitutionError) -> &'static str {
    match error {
        SubstitutionError::NoneLeft => "No changes left.",
        SubstitutionError::NotOnPitch => "That player is not on the pitch.",
        SubstitutionError::NotOnBench => "That player is not on the bench.",
        SubstitutionError::Keeper => "A goalkeeper can only be replaced by a goalkeeper.",
    }
}

/// Whether an event counts for a side in a row of the stats.
type CountsFor = fn(&Event, Side) -> bool;

/// Shots, corners, fouls and cards so far.
fn stats_ui(ui: &mut egui::Ui, home: &str, away: &str, live: &LiveMatch) {
    ui.heading("Stats");
    let count = |side: Side, f: CountsFor| live.events().iter().filter(|e| f(e, side)).count();
    let rows: [(&str, CountsFor); 6] = [
        ("Shots", |e, s| matches!(e, Event::Shot { side, .. } if *side == s)),
        ("On target", |e, s| matches!(e, Event::Shot { side, on_target: true, .. } if *side == s)),
        ("Corners", |e, s| matches!(e, Event::Corner { side, .. } if *side == s)),
        ("Fouls", |e, s| matches!(e, Event::Foul { side, .. } if *side == s)),
        ("Yellow cards", |e, s| matches!(e, Event::Yellow { side, .. } if *side == s)),
        ("Red cards", |e, s| matches!(e, Event::Red { side, .. } if *side == s)),
    ];
    egui::Grid::new("live_stats").striped(true).show(ui, |ui| {
        ui.label("");
        ui.label(RichText::new(home).strong());
        ui.label(RichText::new(away).strong());
        ui.end_row();
        for (label, f) in rows {
            ui.label(label);
            ui.label(count(Side::Home, f).to_string());
            ui.label(count(Side::Away, f).to_string());
            ui.end_row();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::{Formation, MatchInput, MatchPlayer, MatchTeam, Position, ShotBasedParts, DEFAULT_SHOT_BASED_PARTS};

    fn team(first_id: u32) -> MatchTeam {
        let player = |id, position| MatchPlayer { id, position, rating: 70, ratings: [70; Position::COUNT], finishing: 70, creativity: 70, aggression: 60, stamina: 70, start_tiredness: 0.0 };
        let slots = Formation::FourFourTwo.slots();
        MatchTeam { players: std::array::from_fn(|i| player(first_id + i as u32, slots[i])), bench: std::array::from_fn(|i| player(first_id + 20 + i as u32, slots[i])) }
    }

    fn state() -> (State, Rng) {
        let mut rng = Rng::new(1);
        let input = MatchInput { home: team(1), away: team(101), must_have_winner: false, max_substitutions: 5 };
        let parts: ShotBasedParts = DEFAULT_SHOT_BASED_PARTS;
        let mut state = State::new(0, LiveMatch::new(&input, parts, [true, false], &mut rng), Side::Home, Formation::FourFourTwo);
        state.pause_on_goals = false;
        state.playing = true;
        (state, rng)
    }

    #[test]
    fn the_clock_stops_at_half_time_and_at_full_time() {
        let (mut s, mut rng) = state();
        while s.playing {
            play_minute(&mut s, &mut rng);
        }
        assert_eq!(s.live.minute(), 45);
        s.playing = true;
        while s.playing {
            play_minute(&mut s, &mut rng);
        }
        assert!(s.live.is_over() && s.live.minute() == 90);
    }

    #[test]
    fn the_clock_stops_at_a_goal_when_asked_to() {
        let (mut s, mut rng) = state();
        s.pause_on_goals = true;
        while s.playing {
            play_minute(&mut s, &mut rng);
        }
        let goals = s.live.events().iter().filter(|e| matches!(e, Event::Goal { .. })).count();
        assert!(goals == 1 || s.live.minute() == 45, "stopped after {} minutes with {goals} goals", s.live.minute());
    }

    #[test]
    fn two_clicks_swap_two_players_on_the_pitch_and_a_click_on_the_bench_brings_a_substitute_on() {
        let (mut s, _) = state();
        let team = *s.live.team(Side::Home);
        let (winger, striker) = (team.players[6].id, team.players[10].id);
        click(&mut s, winger, &team);
        assert_eq!(s.picked, Some(winger));
        click(&mut s, striker, &team);
        assert_eq!((s.picked, s.message.clone()), (None, None));
        let after = s.live.team(Side::Home);
        assert_eq!((after.players[6].id, after.players[10].id), (striker, winger), "the two traded places");
        // A player on the pitch, then a substitute: a change.
        let team = *s.live.team(Side::Home);
        let sub = team.bench[5].id;
        click(&mut s, team.players[5].id, &team);
        click(&mut s, sub, &team);
        assert_eq!(s.live.team(Side::Home).players[5].id, sub);
        assert_eq!(s.live.substitutions_left(Side::Home), 4);
    }

    #[test]
    fn a_click_on_the_same_player_cancels_and_a_refused_swap_says_why() {
        let (mut s, _) = state();
        let team = *s.live.team(Side::Home);
        click(&mut s, team.players[3].id, &team);
        click(&mut s, team.players[3].id, &team);
        assert_eq!(s.picked, None);
        click(&mut s, team.players[0].id, &team); // the keeper, then an outfield player
        click(&mut s, team.players[5].id, &team);
        assert!(s.message.as_deref().is_some_and(|m| m.contains("goalkeeper")), "{:?}", s.message);
    }
}

