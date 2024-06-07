use bytes::Bytes;
use chrono::{DateTime, Utc};
use eframe::{egui, App, Frame};
use futures_util::stream::StreamExt;
use reqwest::Client;
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error as StdError;
use std::result::Result;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio;

mod led_coords;
mod driver_info;
use crate::led_coords::{LedCoordinate, read_coordinates};
use crate::driver_info::{DriverInfo, get_driver_info};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ApiData {
    x: f64,
    y: f64,
    z: f64,
    driver_number: u32,
    #[serde(deserialize_with = "deserialize_datetime")]
    date: DateTime<Utc>,
    session_key: u32,
    meeting_key: u32,
}

#[derive(Clone, Debug)]
struct RaceData {
    date: DateTime<Utc>,
    driver_number: u32,
    x_led: f64,
    y_led: f64,
}



#[derive(Clone, Debug)]
struct PlotApp {
    coordinates: Vec<LedCoordinate>,
    run_race_data: Vec<RaceData>,
    start_time: Instant,
    race_time: f64, // Elapsed race time in seconds
    race_started: bool,
    data_loading_started: bool,
    data_loaded: bool,
    driver_info: Vec<DriverInfo>,
    current_index: usize,
    led_states: Arc<Mutex<HashMap<(i64, i64), egui::Color32>>>, // Tracks the current state of the LEDs
    last_positions: HashMap<u32, (i64, i64)>, // Last known positions of each driver
    speed: i32,                               // Playback speed multiplier
    completion_sender: Option<async_channel::Sender<()>>,
    completion_receiver: Option<async_channel::Receiver<()>>,
}

impl PlotApp {
    fn new(coordinates: Vec<LedCoordinate>, driver_info: Vec<DriverInfo>) -> PlotApp {
        let (completion_sender, completion_receiver) = async_channel::bounded(1);

        PlotApp {
            coordinates,
            run_race_data: Vec::new(),
            start_time: Instant::now(),
            race_time: 0.0,
            race_started: false,
            data_loading_started: false,
            data_loaded: false,
            driver_info,
            current_index: 0,
            led_states: Arc::new(Mutex::new(HashMap::new())), 
            last_positions: HashMap::new(),
            speed: 1,
            completion_sender: Some(completion_sender),
            completion_receiver: Some(completion_receiver),
        }
    }

    fn reset(&mut self) {
        self.start_time = Instant::now();
        self.race_time = 0.0;
        self.race_started = false;
        self.current_index = 0;
        self.led_states.lock().unwrap().clear(); 
        self.last_positions.clear();
    }

    fn start_race(&mut self) {
        if self.race_started {
            let elapsed = self.start_time.elapsed().as_secs_f64();
            self.race_time = elapsed * self.speed as f64;

            let mut next_index = self.current_index;
            while next_index < self.run_race_data.len() {
                let run_data = &self.run_race_data[next_index];
                let race_data_time =
                    (run_data.date - self.run_race_data[0].date).num_milliseconds() as f64 / 1000.0;
                if race_data_time <= self.race_time {
                    next_index += 1;
                } else {
                    break;
                }
            }

            if next_index != self.current_index {
                self.current_index = next_index;
                let run_race_data_slice = self.run_race_data[..self.current_index].to_vec();
                self.update_led_states(&run_race_data_slice);
            }
        }
    }

    fn update_led_states(&mut self, run_race_data: &[RaceData]) {
        println!("Updating LED states for {} entries", run_race_data.len());
        let mut led_states = self.led_states.lock().unwrap();
        //led_states.clear();

        for run_data in run_race_data.iter() {
            let coord_key = (
                PlotApp::scale_f64(run_data.x_led, 1_000_000),
                PlotApp::scale_f64(run_data.y_led, 1_000_000),
            );

            self.last_positions
                .insert(run_data.driver_number, coord_key);
            println!("Updated last_positions: {:?}", self.last_positions);
        }

        for (&driver_number, &position) in &self.last_positions {
            let color = self
                .driver_info
                .iter()
                .find(|&driver| driver.number == driver_number)
                .map_or(egui::Color32::WHITE, |driver| driver.color);
            println!("LED position: {:?}, Color: {:?}", position, color);
            led_states.insert(position, color);

            if led_states.is_empty() {
                println!("update_led_states -- LED STATES EMPTY!");
            } else {
                println!("update_led_states -- LED STATES FULL!");
            }
        }
    }

    fn scale_f64(value: f64, scale: i64) -> i64 {
        (value * scale as f64) as i64
    }

    async fn fetch_api_data(&mut self) -> Result<(), Box<dyn StdError + Send + Sync>> {
        println!("Starting to load data...");
        let driver_numbers = vec![
            1, 2, 4, 10, 11, 14, 16, 18, 20, 22, 23, 24, 27, 31, 40, 44, 55, 63, 77, 81,
        ];
    
        let mut all_drivers_complete = false;
    
        while !all_drivers_complete {
            let mut handles = Vec::new();
            all_drivers_complete = true;  // Reset to true before checking all drivers
    
            for &driver_number in &driver_numbers {
                let url = format!(
                    "https://api.openf1.org/v1/location?session_key={}&driver_number={}",
                    "9149", driver_number
                );
    
                let mut app_clone = self.clone();
                let sender_clone = self.completion_sender.clone().unwrap();
                handles.push(tokio::spawn(async move {
                    let mut stream = fetch_data_in_chunks(&url, 8 * 1048).await?;
                    let mut buffer = Vec::new();
                    let mut driver_complete = true;
    
                    while let Some(chunk) = stream.next().await {
                        let chunk = chunk?;
                        println!("Received a chunk of data for driver number {}", driver_number);
                        let run_race_data = deserialize_chunk(
                            chunk,
                            &mut buffer,
                            &app_clone.coordinates,
                            usize::MAX, // No limit on rows per driver
                            &sender_clone,
                        ).await?;
    
                        // Sort data by date
                        app_clone.run_race_data.extend(run_race_data);
                        app_clone.run_race_data.sort_by_key(|d| d.date);
    
                        // Visualize the data
                        app_clone.start_race(); 
    
                        if !buffer.is_empty() {
                            driver_complete = false;
                        }
                    }
    
                    if driver_complete {
                        println!("Completed data fetching for driver number {}", driver_number);
                    }
    
                    Ok::<(), Box<dyn StdError + Send + Sync>>(())
                }));
            }
    
            let results = futures::future::join_all(handles).await;
    
            for result in results {
                if let Err(e) = result {
                    eprintln!("Error fetching data: {:?}", e);
                }
            }
    
            // Check if all drivers have completed data fetching
            for driver_number in &driver_numbers {
                let data_complete = Self::check_if_data_complete(driver_number).await;
                if !data_complete {
                    all_drivers_complete = false;
                }
            }
        }
    
        println!("Finished streaming data for all drivers");
        self.data_loaded = true; // Set the flag to true
    
        if let Some(sender) = &self.completion_sender {
            println!("Sending final completion message...");
            let _ = sender.send(()).await;
            println!("Final completion message sent.");
        } else {
            println!("Sender is None.");
        }
    
        Ok(())
    }
    
    
    async fn check_if_data_complete(_driver_number: &u32) -> bool {
        // Implement a mechanism to check if data is complete for the given driver
        // This could be an API call, a flag check, or any other logic specific to your application
        // Return true if data is complete, false otherwise
        // For now, we simulate this check. Adjust the logic as needed.
        // Example: Check some condition to determine if fetching is complete
        false
    }

}

impl App for PlotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.start_race();

        if let Some(receiver) = self.completion_receiver.as_ref() {
            while let Ok(()) = receiver.try_recv() {
                println!("Received completion message!");
                // Force repaint after data is loaded
            }
        }

        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Background,
            egui::Id::new("layer"),
        ));

        let (min_x, max_x) = self
            .coordinates
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), coord| {
                (min.min(coord.x_led), max.max(coord.x_led))
            });
        let (min_y, max_y) = self
            .coordinates
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), coord| {
                (min.min(coord.y_led), max.max(coord.y_led))
            });

        let width = max_x - min_x;
        let height = max_y - min_y;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.separator();
                ui.label(format!(
                    "Race Time: {:02}:{:02}:{:05.2}",
                    (self.race_time / 3600.0).floor() as u32,
                    ((self.race_time % 3600.0) / 60.0).floor() as u32,
                    self.race_time % 60.0
                ));
                ui.separator();

                if ui.button("START").clicked() {
                    if !self.data_loading_started {
                        println!("Start button clicked, beginning data loading...");
                        self.data_loading_started = true;
                        self.race_started = true;
                        self.start_time = Instant::now();
                        let mut app_clone = self.clone();
                        let sender = self.completion_sender.clone().unwrap();
                        tokio::spawn(async move {
                            println!("Spawning data loading task...");
                            app_clone.fetch_api_data().await.unwrap();
                            let _ = sender.send(()).await; // Notify completion
                            println!("Data loading task completed.");
                        });
                    }
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
                style
                    .text_styles
                    .get_mut(&egui::TextStyle::Body)
                    .unwrap()
                    .size = 8.0;

                for driver in &self.driver_info {
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "{}: {} ({})",
                            driver.number, driver.name, driver.team
                        ));
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(ui.cursor().min, egui::vec2(5.0, 5.0)),
                            0.0,
                            driver.color,
                        );
                        ui.add_space(5.0);
                    });
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            for coord in &self.coordinates {
                let norm_x = ((coord.x_led - min_x) / width) as f32 * (ui.available_width() - 60.0);
                let norm_y = (ui.available_height() - 60.0)
                    - (((coord.y_led - min_y) / height) as f32 * (ui.available_height() - 60.0));

                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(norm_x + 30.0, norm_y + 30.0),
                        egui::vec2(20.0, 20.0),
                    ),
                    egui::Rounding::same(0.0),
                    egui::Color32::BLACK,
                );
            }

            let led_states = self.led_states.lock().unwrap();

            //println!("EGUI - led_state: {:?}", led_states);

            for ((x, y), color) in &*led_states {
                let norm_x = ((*x as f64 / 1_000_000.0 - min_x) / width) as f32
                    * (ui.available_width() - 60.0);
                let norm_y = (ui.available_height() - 60.0)
                    - (((*y as f64 / 1_000_000.0 - min_y) / height) as f32
                        * (ui.available_height() - 60.0));

                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(norm_x + 30.0, norm_y + 30.0),
                        egui::vec2(20.0, 20.0),
                    ),
                    egui::Rounding::same(0.0),
                    *color,
                );
            }
        });

        ctx.request_repaint();
    }
}



fn generate_nearest_neighbor(
    raw_data: &[ApiData],
    coordinates: &[LedCoordinate],
) -> Vec<RaceData> {
    raw_data
        .iter()
        .filter(|data| data.x != 0.0 || data.y != 0.0) // Filter out (0, 0) coordinates
        .map(|data| {
            let (nearest_coord, _distance) = coordinates
                .iter()
                .map(|coord| {
                    let distance =
                        ((data.x - coord.x_led).powi(2) + (data.y - coord.y_led).powi(2)).sqrt();
                    (coord, distance)
                })
                .min_by(|(_, dist_a), (_, dist_b)| {
                    dist_a
                        .partial_cmp(dist_b)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap();

            RaceData {
                date: data.date,
                driver_number: data.driver_number,
                x_led: nearest_coord.x_led,
                y_led: nearest_coord.y_led,
            }
        })
        .collect()
}


async fn fetch_data_in_chunks(
    url: &str,
    _chunk_size: usize,
) -> Result<
    impl futures_util::stream::Stream<Item = Result<Bytes, reqwest::Error>>,
    Box<dyn StdError + Send + Sync>,
> {
    let client = Client::new();
    let resp = client.get(url).send().await?.error_for_status()?;
    let stream = resp.bytes_stream();
    Ok(stream)
}

async fn deserialize_chunk(
    chunk: Bytes,
    buffer: &mut Vec<u8>,
    coordinates: &[LedCoordinate],
    max_rows: usize,
    sender: &async_channel::Sender<()>,
) -> Result<Vec<RaceData>, Box<dyn StdError + Send + Sync>> {
    buffer.extend_from_slice(&chunk);

    let mut run_race_data = Vec::new();
    let mut rows_processed = 0;

    // Convert the buffer to a string for processing
    let buffer_str = String::from_utf8_lossy(&buffer);
    let mut start_pos = 0;

    // Process the buffer string to find complete JSON objects
    while let Some(end_pos) = buffer_str[start_pos..].find("},{") {
        // Extract the JSON object slice and remove brackets
        let json_slice = &buffer_str[start_pos..start_pos + end_pos + 1];
        let json_slice = json_slice.trim_start_matches('[').trim_end_matches(']');

        // Ensure the slice starts with '{'
        let json_slice = if !json_slice.starts_with('{') {
            if start_pos > 0 {
                buffer_str[start_pos - 1..start_pos + end_pos + 1].to_string()
            } else {
                // Prepend '{' if start_pos is 0 and it doesn't start with '{'
                let mut json_str = String::from("{");
                json_str.push_str(json_slice);
                json_str
            }
        } else {
            json_slice.to_string()
        };

        match serde_json::from_str::<ApiData>(&json_slice) {
            Ok(location_data) => {
                let new_run_race_data = generate_nearest_neighbor(&[location_data], coordinates);

                rows_processed += new_run_race_data.len();
                run_race_data.extend(new_run_race_data);
                start_pos += end_pos + 3; // Move past the processed part
                if rows_processed >= max_rows {
                    println!("Reached max rows limit: {}", max_rows);
                    break;
                }
            }
            Err(e) => {
                println!("Failed to deserialize ApiData: {:?}", e);
                break; // Break the loop if we can't deserialize a complete object
            }
        }
    }

    // Retain the unprocessed part of the buffer
    *buffer = buffer_str[start_pos..].as_bytes().to_vec();

    // Check if the remaining buffer contains a complete JSON object and process it
    if let Ok(location_data) = serde_json::from_slice::<ApiData>(&buffer) {
        let new_run_race_data = generate_nearest_neighbor(&[location_data], coordinates);
        run_race_data.extend(new_run_race_data);
        *buffer = Vec::new(); // Clear the buffer after processing
    }

    // Send completion message after processing the chunk
    let _ = sender.send(()).await;

    Ok(run_race_data)
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError>> {
    let coordinates = read_coordinates()?;
    let driver_info = get_driver_info();

    let app = PlotApp::new(coordinates, driver_info);

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "F1-LED-CIRCUIT SIMULATION",
        native_options,
        Box::new(|_cc| Box::new(app)),
    )?;

    Ok(())
}
