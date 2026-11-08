//! The live match window: your match played a minute at a time. The clock runs at a speed you pick, stops by itself at
//! half time and full time (and at goals if you ask), and while it is stopped you can make changes from the bench.
use crate::{GREEN, RED};
use eframe::egui::{self, Color32, ProgressBar, RichText};
use engine::{Event, LiveMatch, PlayerId, Rng, Side, SubstitutionError, World};
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
const FEED_HEIGHT: f32 = 420.0;

/// A match being watched.
pub struct State {
    fixture: usize,
    live: LiveMatch,
    /// Your side.
    side: Side,
    playing: bool,
    speed: usize,
    pause_on_goals: bool,
    /// Real seconds since the last minute was played.
    clock: f32,
    /// The player to take off and the substitute to bring on, picked in the lists.
    off: Option<PlayerId>,
    on: Option<PlayerId>,
    /// Why the last change was refused.
    message: Option<String>,
}

impl State {
    pub fn new(fixture: usize, live: LiveMatch, side: Side) -> State {
        State { fixture, live, side, playing: false, speed: DEFAULT_SPEED, pause_on_goals: true, clock: 0.0, off: None, on: None, message: None }
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
    ui.columns(3, |columns| {
        feed_ui(&mut columns[0], world, s);
        team_ui(&mut columns[1], world, s);
        stats_ui(&mut columns[2], home, away, &s.live);
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

/// Your eleven with how fresh each is, your bench, and the change you have picked.
fn team_ui(ui: &mut egui::Ui, world: &World, s: &mut State) {
    let side = s.side;
    let name = |id: PlayerId| world.players.get(&id).map_or("?".to_string(), |p| p.name.clone());
    let team = *s.live.team(side);
    let tiredness = s.live.tiredness(side);
    ui.heading("Your team");
    ui.label(format!("Changes left: {}", s.live.substitutions_left(side)));
    ui.label(RichText::new("On the pitch (click to take off)").strong());
    egui::Grid::new("live_players").show(ui, |ui| {
        for (slot, p) in team.players.iter().enumerate() {
            if p.is_empty() {
                ui.label("");
                ui.label(RichText::new("sent off").color(RED));
                ui.end_row();
                continue;
            }
            ui.label(p.position.code());
            if ui.selectable_label(s.off == Some(p.id), format!("{}  {}", name(p.id), p.rating)).clicked() {
                s.off = if s.off == Some(p.id) { None } else { Some(p.id) };
            }
            ui.add(ProgressBar::new((1.0 - tiredness[slot]) as f32).desired_width(70.0).text(format!("{:.0}%", 100.0 * (1.0 - tiredness[slot]))));
            ui.end_row();
        }
    });
    ui.add_space(6.0);
    ui.label(RichText::new("Bench (click to bring on)").strong());
    for p in team.bench.iter().filter(|p| !p.is_empty()) {
        if ui.selectable_label(s.on == Some(p.id), format!("{}  {}  {}", p.position.code(), name(p.id), p.rating)).clicked() {
            s.on = if s.on == Some(p.id) { None } else { Some(p.id) };
        }
    }
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        let ready = s.off.is_some() && s.on.is_some() && !s.playing && !s.live.is_over();
        if ui.add_enabled(ready, egui::Button::new("Make change")).clicked() {
            if let (Some(off), Some(on)) = (s.off, s.on) {
                match s.live.substitute(side, off, on) {
                    Ok(()) => {
                        (s.off, s.on, s.message) = (None, None, None);
                    }
                    Err(e) => s.message = Some(refusal(e).to_string()),
                }
            }
        }
        if s.playing && !s.live.is_over() {
            ui.label("Pause the match to make a change.");
        }
    });
    if let Some(message) = &s.message {
        ui.colored_label(RED, message);
    }
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
    use engine::{Formation, MatchInput, MatchPlayer, MatchTeam, ShotBasedParts, DEFAULT_SHOT_BASED_PARTS};

    fn team(first_id: u32) -> MatchTeam {
        let player = |id, position| MatchPlayer { id, position, rating: 70, finishing: 70, creativity: 70, aggression: 60, stamina: 70 };
        let slots = Formation::FourFourTwo.slots();
        MatchTeam { players: std::array::from_fn(|i| player(first_id + i as u32, slots[i])), bench: std::array::from_fn(|i| player(first_id + 20 + i as u32, slots[i])) }
    }

    fn state() -> (State, Rng) {
        let mut rng = Rng::new(1);
        let input = MatchInput { home: team(1), away: team(101), must_have_winner: false, max_substitutions: 5 };
        let parts: ShotBasedParts = DEFAULT_SHOT_BASED_PARTS;
        let mut state = State::new(0, LiveMatch::new(&input, parts, [true, false], &mut rng), Side::Home);
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
}
