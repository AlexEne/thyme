//! A demo RPG character sheet application.  This file contains the common code including
//! ui layout and logic.  `demo_glium.rs` and `demo_wgpu.rs` both use this file.
//! This file contains example uses of many of Thyme's features.

use std::collections::HashMap;
use thyme::{Frame, bench, ShowElement};

#[derive(Default)]
pub struct Party {
    members: Vec<Character>,
    editing_index: Option<usize>,
    reload_assets: bool,
}

impl Party {
    pub fn take_reload_assets(&mut self) -> bool {
        let result = self.reload_assets;
        self.reload_assets = false;
        result
    }
}

const MIN_AGE: f32 = 18.0;
const DEFAULT_AGE: f32 = 25.0;
const MAX_AGE: f32 = 50.0;
const INITIAL_GP: u32 = 100;
const MIN_STAT: u32 = 3;
const MAX_STAT: u32 = 18;
const STAT_POINTS: u32 = 75;

struct Character {
    name: String,
    age: f32,
    stats: HashMap<Stat, u32>,

    race: Race,
    gp: u32,
    items: Vec<Item>,
}

impl Character {
    fn generate(index: usize) -> Character {
        Character {
            name: format!("Charname {}", index),
            age: DEFAULT_AGE,
            stats: HashMap::default(),
            gp: INITIAL_GP,
            items: Vec::default(),
            race: Race::default(),
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum Race {
    Human,
    Elf,
    Dwarf,
    Halfling,
}

impl Race {
    fn all() -> &'static [Race] {
        use Race::*;
        &[Human, Elf, Dwarf, Halfling]
    }
}

impl std::fmt::Display for Race {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Default for Race {
    fn default() -> Self {
        Race::Human
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum Stat {
    Strength,
    Dexterity,
    Constitution,
    Intelligence,
    Wisdom,
    Charisma,
}

impl Stat {
    fn iter() -> impl Iterator<Item=Stat> + 'static {
        use Stat::*;
        [Strength, Dexterity, Constitution, Intelligence, Wisdom, Charisma].iter().copied()
    }
}

#[derive(Clone)]
struct Item {
    name: &'static str,
    price: u32,
}

const ITEMS: [Item; 3] = [
    Item { name: "Sword", price: 50 },
    Item { name: "Shield", price: 20 },
    Item { name: "Torch", price: 2 }
];

/// The function to build the Thyme user interface.  Called once each frame.  This
/// example demonstrates a combination of Rust layout and styling as well as use
/// of the theme definition file, loaded above
pub fn build_ui(ui: &mut Frame, party: &mut Party) {
    ui.label("bench", format!(
        "{}\n{}\n{}",
        bench::report("thyme"),
        bench::report("frame"),
        bench::report("draw"),
    ));

    if ui.child("reload").clicked {
        party.reload_assets = true;
    }

    ui.start("party_window")
    .window("party_window")
    .with_close_button(false)
    .moveable(false)
    .resizable(false)
    .children(|ui| {
        ui.scrollpane("members_panel", "party_content", |ui| {
            party_members_panel(ui, party);
        });
    });

    if let Some(index) = party.editing_index {
        let character = &mut party.members[index];

        ui.window("character_window", |ui| {
            ui.scrollpane("pane", "character_content", |ui| {
                ui.start("name_panel")
                .children(|ui| {
                    if let Some(new_name) = ui.input_field("name_input", "name_input") {
                        character.name = new_name;
                    }
                });

                ui.gap(10.0);
                ui.label("age_label", format!("Age: {}", character.age.round() as u32));
                if let Some(age) = ui.horizontal_slider("age_slider", MIN_AGE, MAX_AGE, character.age) {
                    character.age = age;
                }

                ui.gap(10.0);

                if let Some(race) = ui.combo_box("race_selector", "race_selector", character.race, Race::all()) {
                    character.race = *race;
                }
    
                ui.gap(10.0);
    
                ui.start("stats_panel")
                .children(|ui| {
                    stats_panel(ui, character);
                });
                
                ui.gap(10.0);
    
                ui.start("inventory_panel")
                .children(|ui| {
                    inventory_panel(ui, character);
                });
            });
        });

        ui.window("item_picker", |ui| {
            let display_size = ui.display_size();

            ui.start("greyed_out").unclip().size(display_size.x, display_size.y).screen_pos(0.0, 0.0).finish();
            item_picker(ui, character);
        });
    }
}

fn party_members_panel(ui: &mut Frame, party: &mut Party) {
    for (index, member) in party.members.iter_mut().enumerate() {
        let clicked = ui.start("filled_slot_button")
        .text(&member.name)
        .active(Some(index) == party.editing_index)
        .finish().clicked;

        if clicked {
            set_active_character(ui, member);
            party.editing_index = Some(index);
        }
    }

    if ui.start("add_character_button").finish().clicked {
        let new_member = Character::generate(party.members.len());
        set_active_character(ui, &new_member);
        party.members.push(new_member);
        party.editing_index = Some(party.members.len() - 1);
    }
}

fn set_active_character(ui: &mut Frame, character: &Character) {
    ui.open("character_window");
    ui.modify("name_input", |state| {
        state.text = Some(character.name.clone());
    });
    ui.close("item_picker");
}

fn stats_panel(ui: &mut Frame, character: &mut Character) {
    let points_used: u32 = character.stats.values().sum();
    let points_available: u32 = STAT_POINTS - points_used;

    ui.child("title");

    let frac = ((ui.cur_time_millis() - ui.base_time_millis("stat_roll")) as f32 / 1000.0).min(1.0);

    let roll = ui.start("roll_button")
    .enabled(frac > 0.99)
    .children(|ui| {
        ui.progress_bar("progress_bar", frac);
    });

    if roll.clicked {
        ui.set_base_time_now("stat_roll");
    }

    for stat in Stat::iter() {
        let value = character.stats.entry(stat).or_insert(10);

        ui.start("stat_panel")
        .children(|ui| {
            ui.label("label", format!("{:?}", stat));

            let clicked = ui.start("decrease").enabled(*value > MIN_STAT).finish().clicked;
            if clicked {
                *value -= 1;
            }

            ui.label("value", format!("{}", *value));
            
            let clicked = ui.start("increase").enabled(points_available > 0 && *value < MAX_STAT).finish().clicked;
            if clicked {
                *value = 18.min(*value + 1);
            }
        }); 
    }

    ui.label("points_available", format!("Points Remaining: {}", points_available));
}

fn item_picker(ui: &mut Frame, character: &mut Character) {
    for item in ITEMS.iter() {
        let clicked = ui.start("item_button")
        .enabled(character.gp >= item.price)
        .children(|ui| {
            ui.label("name", item.name);
            // TODO icon image
            ui.child("icon");
            ui.label("price", format!("{} Gold", item.price));
        }).clicked;

        if clicked {
            character.gp -= item.price;
            character.items.push(item.clone());
            ui.close("item_picker");
        }
    }
}

fn inventory_panel(ui: &mut Frame, character: &mut Character) {
    ui.child("title");
    ui.start("top_panel")
    .children(|ui| {
        if ui.child("buy").clicked {
            ui.open_modal("item_picker");
        }

        ui.label("gold", format!("{} Gold", character.gp));
    });
    
    ui.start("items_panel")
    .scrollpane("items_content")
    .show_vertical_scrollbar(ShowElement::Always)
    .children(|ui| {
        items_panel(ui, character);
    });
}

fn items_panel(ui: &mut Frame, character: &mut Character) {
    let mut sell = None;
    for (index, item) in character.items.iter().enumerate() {
        // TODO tooltip that says remove item
        if ui.button("item_button", item.name).clicked {
            sell = Some(index);
        }
    }

    if let Some(index) = sell {
        let item = character.items.remove(index);
        character.gp += item.price;
    }
}