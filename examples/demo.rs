//! A demo RPG character sheet application.  This file contains the common code including
//! ui layout and logic.  `demo_glium.rs` and `demo_wgpu.rs` both use this file.
//! This file contains example uses of many of Thyme's features.

use std::path::Path;
use std::collections::HashMap;
use thyme::{Context, ContextBuilder, Frame, bench, ShowElement, Renderer};

pub fn register_assets(context_builder: &mut ContextBuilder) {
    // register resources in thyme by reading from files.  this enables live reload.
    context_builder.register_theme_from_files(
        &[
            Path::new("examples/data/themes/base.yml"),
            Path::new("examples/data/themes/demo.yml"),
            // note we dynamically add to this list later if the user selects a new theme
        ],
        serde_yaml::from_str::<serde_yaml::Value>
    ).unwrap();
    context_builder.register_texture_from_file("pixel", Path::new("examples/data/images/pixel.png"));
    context_builder.register_texture_from_file("fantasy", Path::new("examples/data/images/fantasy.png"));
    context_builder.register_texture_from_file("transparent", Path::new("examples/data/images/transparent.png"));
    context_builder.register_texture_from_file("golden", Path::new("examples/data/images/golden.png"));
    context_builder.register_font_from_file("Roboto-Medium", Path::new("examples/data/fonts/Roboto-Medium.ttf"));
}

#[derive(Debug, Copy, Clone)]
enum ThemeChoice {
    Pixels,
    Fantasy,
    Transparent,
    Golden,
}

const THEME_CHOICES: [ThemeChoice; 4] = [
    ThemeChoice::Pixels, ThemeChoice::Fantasy, ThemeChoice::Transparent, ThemeChoice::Golden
];

impl ThemeChoice {
    fn path(self) -> Option<&'static str> {
        match self {
            ThemeChoice::Fantasy => Some("examples/data/themes/fantasy.yml"),
            ThemeChoice::Pixels => None,
            ThemeChoice::Transparent => Some("examples/data/themes/transparent.yml"),
            ThemeChoice::Golden => Some("examples/data/themes/golden.yml"),
        }
    }
}

impl Default for ThemeChoice {
    fn default() -> Self {
        ThemeChoice::Pixels
    }
}

impl std::fmt::Display for ThemeChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Default)]
pub struct Party {
    members: Vec<Character>,
    editing_index: Option<usize>,

    live_reload_disabled: bool,
    reload_assets: bool,
    old_theme_choice: Option<ThemeChoice>,
    theme_choice: ThemeChoice,
}

impl Party {
    pub fn check_context_changes<R: Renderer>(&mut self, context: &mut Context, renderer: &mut R) {
        if let Some(old_choice) = self.old_theme_choice.take() {
            if let Some(path) = old_choice.path() {
                context.remove_theme_file(path);
            }

            if let Some(path) = self.theme_choice.path() {
                context.add_theme_file(path);
            }
        }

        if self.reload_assets {
            if let Err(e) = context.rebuild_all(renderer) {
                log::error!("Unable to rebuild theme: {}", e);
            }
            self.reload_assets = false;
        } else if !self.live_reload_disabled {
            if let Err(e) = context.check_live_reload(renderer) {
                log::error!("Unable to live reload theme: {}", e);
            }
        }
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

    ui.start("theme_panel").children(|ui| {
        if ui.start("live_reload").active(!party.live_reload_disabled).finish().clicked {
            party.live_reload_disabled = !party.live_reload_disabled;
        }

        if let Some(choice) = ui.combo_box("theme_choice", "theme_choice", &party.theme_choice, &THEME_CHOICES) {
            party.old_theme_choice = Some(party.theme_choice);
            party.theme_choice = *choice;
            party.reload_assets = true;
        }
    });

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
                    if let Some(new_name) = ui.input_field("name_input", "name_input", None) {
                        character.name = new_name;
                    }
                });

                ui.gap(10.0);
                ui.label("age_label", format!("Age: {}", character.age.round() as u32));
                if let Some(age) = ui.horizontal_slider("age_slider", MIN_AGE, MAX_AGE, character.age) {
                    character.age = age;
                }

                ui.gap(10.0);

                if let Some(race) = ui.combo_box("race_selector", "race_selector", &character.race, Race::all()) {
                    character.race = *race;
                }
    
                ui.gap(10.0);
    
                ui.tree("stats_panel", "stats_panel", true,
                |ui| {
                    ui.child("title");
                },|ui| {
                    stats_panel(ui, character);
                });
                
                ui.gap(10.0);
    
                ui.tree("inventory_panel", "inventory_panel", true,
                |ui| {
                    ui.child("title");
                }, |ui| {
                    inventory_panel(ui, character);
                });
            });
        });

        ui.window("item_picker", |ui| {
            let display_size = ui.display_size();

            ui.start("greyed_out")
            .unclip()
            .unparent()
            .size(display_size.x, display_size.y)
            .screen_pos(0.0, 0.0).finish();

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

        ui.tree(
        "stat_panel",
        &format!("stat_panel_{:?}", stat),
        false,
        |ui| {
            ui.label("label", format!("{:?}", stat));

            match ui.spinner("spinner", *value, MIN_STAT, if points_available == 0 { *value } else { MAX_STAT }) {
                1 => *value += 1,
                -1 => *value -= 1,
                _ => (),
            }
        }, |ui| {
            ui.child("description");
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
        let result = ui.button("item_button", item.name);
        if result.clicked {
            sell = Some(index);
        }
        
        if result.hovered {
            // manually specify a tooltip
            ui.tooltip("tooltip", "Remove Item");
        }
    }

    if let Some(index) = sell {
        let item = character.items.remove(index);
        character.gp += item.price;
    }
}