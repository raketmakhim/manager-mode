//! The summer's deals that involve your club, shown before the next season starts. Each one can be gone ahead with or
//! turned down; a turned-down player stays where he is.
use eframe::egui::{self, RichText};
use engine::{ClubId, Transfer, TransferKind, Window, World};

/// "€4.5m", or "free" when nothing is paid.
fn fee_text(fee: u32) -> String {
    if fee == 0 {
        "free".to_string()
    } else {
        format!("€{:.1}m", fee as f64 / 1_000_000.0)
    }
}

/// One line describing a deal from your club's side: "Sell Name (84) to Club for €4.5m".
fn deal_text(world: &World, club: ClubId, t: &Transfer) -> String {
    let player = &world.players[&t.player];
    let selling = t.from == Some(club);
    let other = if selling { t.to } else { t.from }.map_or("?", |c| world.clubs[&c].name.as_str());
    let what = match (selling, t.kind) {
        (true, TransferKind::Permanent) => "Sell",
        (true, TransferKind::Loan) => "Loan out",
        (false, TransferKind::Permanent) => "Sign",
        (false, TransferKind::Loan) => "Loan in",
    };
    let direction = if selling { "to" } else { "from" };
    format!("{what} {} ({}, {}) {direction} {other} for {}", player.name, player.overall, player.positions.first().map_or("", |p| p.code()), fee_text(t.fee))
}

/// Lists the deals for `club` with a "Go ahead" box each. True when the user confirms the list.
pub fn ui(ui: &mut egui::Ui, world: &World, club: ClubId, window: &mut Window) -> bool {
    ui.heading("Transfers");
    ui.label("Real moves of this summer that involve your club. Untick a deal to keep the player where he is. Everyone else's moves happen too.");
    ui.add_space(6.0);
    for index in window.questions_for(club) {
        let mut go_ahead = !window.is_declined(index);
        if ui.checkbox(&mut go_ahead, deal_text(world, club, &window.transfers()[index])).changed() {
            window.set_declined(index, !go_ahead);
        }
    }
    ui.add_space(8.0);
    ui.button(RichText::new("Confirm and start the next season").strong()).clicked()
}
