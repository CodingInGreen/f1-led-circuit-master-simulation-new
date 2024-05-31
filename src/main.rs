use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde::de::{self, Deserializer};
use std::error::Error as StdError;
use std::result::Result;
use std::sync::Arc;
use std::sync::Mutex;
use tokio;
use chrono::{DateTime, Utc};
use csv::ReaderBuilder;
use eframe::{egui, App, Frame};
use std::time::Instant;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
struct LocationData {
    x: f64,
    y: f64,
    #[serde(deserialize_with = "deserialize_datetime")]
    date: DateTime<Utc>,
    driver_number: u32,
}

#[derive(Debug, Deserialize)]
struct LedCoordinate {
    x_led: f64,
    y_led: f64,
}

#[derive(Debug)]
struct RunRace {
    date: DateTime<Utc>,
    driver_number: u32,
    x_led: f64,
    y_led: f64,
}

#[derive(Debug)]
struct DriverInfo {
    number: u32,
    name: &'static str,
    team: &'static str,
    color: egui::Color32,
}

struct PlotApp {
    coordinates: Vec<LedCoordinate>,
    run_race_data: Vec<RunRace>,
    start_time: Instant,
    race_time: f64, // Elapsed race time in seconds
    race_started: bool,
    colors: HashMap<u32, egui::Color32>,
    driver_info: Vec<DriverInfo>,
    current_index: usize,
    led_states: HashMap<(i64, i64), egui::Color32>,  // Tracks the current state of the LEDs
    last_positions: HashMap<u32, (i64, i64)>,  // Last known positions of each driver
    speed: i32, // Playback speed multiplier
}

impl PlotApp {
    fn new(coordinates: Vec<LedCoordinate>, run_race_data: Vec<RunRace>, colors: HashMap<u32, egui::Color32>, driver_info: Vec<DriverInfo>) -> Self {
        Self {
            coordinates,
            run_race_data,
            start_time: Instant::now(),
            race_time: 0.0,
            race_started: false,
            colors,
            driver_info,
            current_index: 0,
            led_states: HashMap::new(), // Initialize empty LED state tracking
            last_positions: HashMap::new(), // Initialize empty last positions hashmap
            speed: 1,
        }
    }

    fn reset(&mut self) {
        self.start_time = Instant::now();
        self.race_time = 0.0;
        self.race_started = false;
        self.current_index = 0;
        self.led_states.clear(); // Reset LED states
        self.last_positions.clear(); // Reset last positions
    }

    fn update_race(&mut self) {
        if self.race_started {
            let elapsed = self.start_time.elapsed().as_secs_f64();
            self.race_time = elapsed * self.speed as f64;

            let mut next_index = self.current_index;
            while next_index < self.run_race_data.len() {
                let run_data = &self.run_race_data[next_index];
                let race_data_time = (run_data.date - self.run_race_data[0].date).num_milliseconds() as f64 / 1000.0;
                if race_data_time <= self.race_time {
                    next_index += 1;
                } else {
                    break;
                }
            }

            self.current_index = next_index;
            self.update_led_states();
        }
    }

    fn update_led_states(&mut self) {
        self.led_states.clear();

        for run_data in &self.run_race_data[..self.current_index] {
            let coord_key = (
                Self::scale_f64(run_data.x_led, 1_000_000),
                Self::scale_f64(run_data.y_led, 1_000_000),
            );

            // Update the last known position of the driver
            self.last_positions.insert(run_data.driver_number, coord_key);
        }

        // Update the LED states for all known positions
        for (&driver_number, &position) in &self.last_positions {
            let color = self.colors.get(&driver_number).copied().unwrap_or(egui::Color32::WHITE);
            self.led_states.insert(position, color);
        }
    }

    fn scale_f64(value: f64, scale: i64) -> i64 {
        (value * scale as f64) as i64
    }
}

impl App for PlotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.update_race();

        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Background, egui::Id::new("my_layer")));

        let (min_x, max_x) = self.coordinates.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), coord| {
            (min.min(coord.x_led), max.max(coord.x_led))
        });
        let (min_y, max_y) = self.coordinates.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), coord| {
            (min.min(coord.y_led), max.max(coord.y_led))
        });

        let width = max_x - min_x;
        let height = max_y - min_y;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.separator();
                ui.label(format!(
                    "Race Time: {:02}:{:02}:{:05.2}",
                    (self.race_time / 3600.0).floor() as u32, // hours
                    ((self.race_time % 3600.0) / 60.0).floor() as u32, // minutes
                    self.race_time % 60.0 // seconds with milliseconds
                ));
                ui.separator();

                if ui.button("START").clicked() {
                    self.race_started = true;
                    self.start_time = Instant::now();
                    self.current_index = 0;
                    self.led_states.clear(); // Clear LED states when race starts
                }
                if ui.button("STOP").clicked() {
                    self.reset();
                }

                ui.label("PLAYBACK SPEED");
                ui.add(egui::Slider::new(&mut self.speed, 1..=5));

            });
        });

        egui::SidePanel::right("legend_panel").show(ctx, |ui| {
            ui.vertical(|ui| {
                let style = ui.style_mut();
                style.text_styles.get_mut(&egui::TextStyle::Body).unwrap().size = 8.0; // Set the font size to 8.0 (or any other size you prefer)

                for driver in &self.driver_info {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}: {} ({})", driver.number, driver.name, driver.team));
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(
                                ui.cursor().min,
                                egui::vec2(5.0, 5.0),
                            ),
                            0.0,
                            driver.color,
                        );
                        ui.add_space(5.0); // Space between legend items
                    });
                }
            });
        });

        egui::CentralPanel::default()
            .show(ctx, |ui| {
                for coord in &self.coordinates {
                    let norm_x = ((coord.x_led - min_x) / width) as f32 * (ui.available_width() - 60.0); // Adjust for left/right margin
                    let norm_y = (ui.available_height() - 60.0) - (((coord.y_led - min_y) / height) as f32 * (ui.available_height() - 60.0)); // Adjust for top/bottom margin

                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(norm_x + 30.0, norm_y + 30.0), // Adjust position to include margins
                            egui::vec2(20.0, 20.0),
                        ),
                        egui::Rounding::same(0.0),
                        egui::Color32::BLACK,
                    );
                }

                for ((x, y), color) in &self.led_states {
                    let norm_x = ((*x as f64 / 1_000_000.0 - min_x) / width) as f32 * (ui.available_width() - 60.0); // Adjust for left/right margin
                    let norm_y = (ui.available_height() - 60.0) - (((*y as f64 / 1_000_000.0 - min_y) / height) as f32 * (ui.available_height() - 60.0)); // Adjust for top/bottom margin

                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(norm_x + 30.0, norm_y + 30.0), // Adjust position to include margins
                            egui::vec2(20.0, 20.0),
                        ),
                        egui::Rounding::same(0.0),
                        *color,
                    );
                }
            });

        ctx.request_repaint(); // Request the GUI to repaint
    }
}

fn main() -> Result<(), Box<dyn StdError>> {
    let coordinates = read_coordinates("led_coords.csv")?;

    // Initialize the runtime for async execution
    let runtime = tokio::runtime::Runtime::new()?;
    let raw_data = runtime.block_on(fetch_data())?;

    let run_race_data = generate_run_race_data(&raw_data, &coordinates);

    let driver_info = vec![
        DriverInfo { number: 1, name: "Max Verstappen", team: "Red Bull", color: egui::Color32::from_rgb(30, 65, 255) },
        DriverInfo { number: 2, name: "Logan Sargeant", team: "Williams", color: egui::Color32::from_rgb(0, 82, 255) },
        DriverInfo { number: 4, name: "Lando Norris", team: "McLaren", color: egui::Color32::from_rgb(255, 135, 0) },
        DriverInfo { number: 10, name: "Pierre Gasly", team: "Alpine", color: egui::Color32::from_rgb(2, 144, 240) },
        DriverInfo { number: 11, name: "Sergio Perez", team: "Red Bull", color: egui::Color32::from_rgb(30, 65, 255) },
        DriverInfo { number: 14, name: "Fernando Alonso", team: "Aston Martin", color: egui::Color32::from_rgb(0, 110, 120) },
        DriverInfo { number: 16, name: "Charles Leclerc", team: "Ferrari", color: egui::Color32::from_rgb(220, 0, 0) },
        DriverInfo { number: 18, name: "Lance Stroll", team: "Aston Martin", color: egui::Color32::from_rgb(0, 110, 120) },
        DriverInfo { number: 20, name: "Kevin Magnussen", team: "Haas", color: egui::Color32::from_rgb(160, 207, 205) },
        DriverInfo { number: 22, name: "Yuki Tsunoda", team: "AlphaTauri", color: egui::Color32::from_rgb(60, 130, 200) },
        DriverInfo { number: 23, name: "Alex Albon", team: "Williams", color: egui::Color32::from_rgb(0, 82, 255) },
        DriverInfo { number: 24, name: "Zhou Guanyu", team: "Stake F1", color: egui::Color32::from_rgb(165, 160, 155) },
        DriverInfo { number: 27, name: "Nico Hulkenberg", team: "Haas", color: egui::Color32::from_rgb(160, 207, 205) },
        DriverInfo { number: 31, name: "Esteban Ocon", team: "Alpine", color: egui::Color32::from_rgb(2, 144, 240) },
        DriverInfo { number: 40, name: "Liam Lawson", team: "AlphaTauri", color: egui::Color32::from_rgb(60, 130, 200) },
        DriverInfo { number: 44, name: "Lewis Hamilton", team: "Mercedes", color: egui::Color32::from_rgb(0, 210, 190) },
        DriverInfo { number: 55, name: "Carlos Sainz", team: "Ferrari", color: egui::Color32::from_rgb(220, 0, 0) },
        DriverInfo { number: 63, name: "George Russell", team: "Mercedes", color: egui::Color32::from_rgb(0, 210, 190) },
        DriverInfo { number: 77, name: "Valtteri Bottas", team: "Stake F1", color: egui::Color32::from_rgb(165, 160, 155) },
        DriverInfo { number: 81, name: "Oscar Piastri", team: "McLaren", color: egui::Color32::from_rgb(255, 135, 0) },
    ];

    let mut colors = HashMap::new();
    colors.insert(1, egui::Color32::from_rgb(30, 65, 255));  // Max Verstappen, Red Bull
    colors.insert(2, egui::Color32::from_rgb(0, 82, 255));   // Logan Sargeant, Williams
    colors.insert(4, egui::Color32::from_rgb(255, 135, 0));  // Lando Norris, McLaren
    colors.insert(10, egui::Color32::from_rgb(2, 144, 240)); // Pierre Gasly, Alpine
    colors.insert(11, egui::Color32::from_rgb(30, 65, 255)); // Sergio Perez, Red Bull
    colors.insert(14, egui::Color32::from_rgb(0, 110, 120)); // Fernando Alonso, Aston Martin
    colors.insert(16, egui::Color32::from_rgb(220, 0, 0));   // Charles Leclerc, Ferrari
    colors.insert(18, egui::Color32::from_rgb(0, 110, 120)); // Lance Stroll, Aston Martin
    colors.insert(20, egui::Color32::from_rgb(160, 207, 205)); // Kevin Magnussen, Haas
    colors.insert(22, egui::Color32::from_rgb(60, 130, 200)); // Yuki Tsunoda, AlphaTauri
    colors.insert(23, egui::Color32::from_rgb(0, 82, 255));  // Alex Albon, Williams
    colors.insert(24, egui::Color32::from_rgb(165, 160, 155)); // Zhou Guanyu, Stake F1
    colors.insert(27, egui::Color32::from_rgb(160, 207, 205)); // Nico Hulkenberg, Haas
    colors.insert(31, egui::Color32::from_rgb(2, 144, 240));   // Esteban Ocon, Alpine
    colors.insert(40, egui::Color32::from_rgb(60, 130, 200));  // Liam Lawson, AlphaTauri
    colors.insert(44, egui::Color32::from_rgb(0, 210, 190));   // Lewis Hamilton, Mercedes
    colors.insert(55, egui::Color32::from_rgb(220, 0, 0));     // Carlos Sainz, Ferrari
    colors.insert(63, egui::Color32::from_rgb(0, 210, 190));   // George Russell, Mercedes
    colors.insert(77, egui::Color32::from_rgb(165, 160, 155)); // Valtteri Bottas, Stake F1
    colors.insert(81, egui::Color32::from_rgb(255, 135, 0));   // Oscar Piastri, McLaren

    let app = PlotApp::new(coordinates, run_race_data, colors, driver_info);

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "F1-LED-CIRCUIT SIMULATION",
        native_options,
        Box::new(|_cc| Box::new(app)),
    )?;

    Ok(())
}

async fn fetch_data() -> Result<Vec<LocationData>, Box<dyn StdError>> {
    let session_key = "9149";
    let driver_numbers = vec![
        1, 2, 4, 10, 11, 14, 16, 18, 20, 22, 23, 24, 27, 31, 40, 44, 55, 63, 77, 81
    ];

    let client = Client::new();
    let mut all_data: Vec<LocationData> = Vec::new();

    for driver_number in driver_numbers {
        let url = format!("https://api.openf1.org/v1/location?session_key={}&driver_number={}", session_key, driver_number);
        let resp = client.get(&url).send().await?;
        if resp.status().is_success() {
            let data: Vec<LocationData> = resp.json().await?;
            all_data.extend(data);
        } else {
            eprintln!("Failed to fetch data for driver {}: HTTP {}", driver_number, resp.status());
        }
    }

    // Sort the data by the date field
    all_data.sort_by_key(|d| d.date);
    Ok(all_data)
}

fn read_coordinates(file_path: &str) -> Result<Vec<LedCoordinate>, Box<dyn StdError>> {
    let mut rdr = ReaderBuilder::new().from_path(file_path)?;
    let mut coordinates = Vec::new();
    for result in rdr.deserialize() {
        let record: LedCoordinate = result?;
        coordinates.push(record);
    }
    Ok(coordinates)
}

fn generate_run_race_data(raw_data: &[LocationData], coordinates: &[LedCoordinate]) -> Vec<RunRace> {
    raw_data.iter().map(|data| {
        let (nearest_coord, _distance) = coordinates.iter()
            .map(|coord| {
                let distance = ((data.x - coord.x_led).powi(2) + (data.y - coord.y_led).powi(2)).sqrt();
                (coord, distance)
            })
            .min_by(|(_, dist_a), (_, dist_b)| dist_a.partial_cmp(dist_b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();

        RunRace {
            date: data.date,
            driver_number: data.driver_number,
            x_led: nearest_coord.x_led,
            y_led: nearest_coord.y_led,
        }
    }).collect()
}

fn deserialize_datetime<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    DateTime::parse_from_rfc3339(&s)
        .map_err(de::Error::custom)
        .map(|dt| dt.with_timezone(&Utc))
}
