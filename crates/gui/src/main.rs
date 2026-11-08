//! Manager Mode window. Tabs: Career (new career, your club's fixtures, squad and competitions) and Player scout (real player history).
//! Usage: gui [history.db]
mod career;
mod competitions;
mod fixtures;
mod leagues;
mod lineup;
mod scout;
mod squad;
mod stats;

use eframe::egui::{self, Color32};
use engine::Connection;

pub const GREEN: Color32 = Color32::from_rgb(70, 190, 90);
pub const RED: Color32 = Color32::from_rgb(230, 80, 80);

#[derive(PartialEq)]
enum Tab {
    Career,
    Scout,
}

struct App {
    conn: Connection,
    tab: Tab,
    career: career::Career,
    scout: scout::Scout,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("tabs").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Career, "Career");
                ui.selectable_value(&mut self.tab, Tab::Scout, "Player scout");
            });
        });
        match self.tab {
            Tab::Career => self.career.ui(ui, &self.conn),
            Tab::Scout => self.scout.ui(ui, &self.conn),
        }
    }
}

fn main() -> eframe::Result {
    let path = std::env::args().nth(1).unwrap_or("history.db".into());
    if !std::path::Path::new(&path).exists() {
        eprintln!("{path} not found. Build it first: cargo run -p import --release");
        std::process::exit(1);
    }
    let conn = Connection::open(&path).expect("open history.db");
    let app = App { career: career::Career::new(&conn), scout: scout::Scout::default(), tab: Tab::Career, conn };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1250.0, 850.0]),
        ..Default::default()
    };
    eframe::run_native("Manager Mode", options, Box::new(|_cc| Ok(Box::new(app))))
}
