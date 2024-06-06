use chrono::{DateTime, Utc};
use eframe::{egui, App, Frame};
use reqwest::Client;
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error as StdError;
use std::result::Result;
use std::time::Instant;
use tokio;
use bytes::Bytes;
use futures_util::stream::StreamExt;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct LocationData {
    x: f64,
    y: f64,
    z: f64,
    driver_number: u32,
    #[serde(deserialize_with = "deserialize_datetime")]
    date: DateTime<Utc>,
    session_key: u32,
    meeting_key: u32,
}

#[derive(Clone, Debug, Deserialize)]
struct LedCoordinate {
    x_led: f64,
    y_led: f64,
}

#[derive(Clone, Debug)]
struct RunRace {
    date: DateTime<Utc>,
    driver_number: u32,
    x_led: f64,
    y_led: f64,
}

#[derive(Clone, Debug)]
struct DriverInfo {
    number: u32,
    name: &'static str,
    team: &'static str,
    color: egui::Color32,
}

#[derive(Clone, Debug)]
struct PlotApp {
    coordinates: Vec<LedCoordinate>,
    run_race_data: Vec<RunRace>,
    start_time: Instant,
    race_time: f64, // Elapsed race time in seconds
    race_started: bool,
    data_loading_started: bool,
    data_loaded: bool,
    driver_info: Vec<DriverInfo>,
    current_index: usize,
    led_states: HashMap<(i64, i64), egui::Color32>, // Tracks the current state of the LEDs
    last_positions: HashMap<u32, (i64, i64)>,       // Last known positions of each driver
    speed: i32,                                     // Playback speed multiplier
    completion_sender: Option<async_channel::Sender<()>>,
    completion_receiver: Option<async_channel::Receiver<()>>,
}

impl PlotApp {
    fn new(
        coordinates: Vec<LedCoordinate>,
        driver_info: Vec<DriverInfo>,
    ) -> PlotApp {
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
            led_states: HashMap::new(), // Initialize empty LED state tracking
            last_positions: HashMap::new(), // Initialize empty last positions hashmap
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
                let race_data_time =
                    (run_data.date - self.run_race_data[0].date).num_milliseconds() as f64 / 1000.0;
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
        
        println!("Updating LED states...");
    
        for run_data in &self.run_race_data[..self.current_index] {
            let coord_key = (
                Self::scale_f64(run_data.x_led, 1_000_000),
                Self::scale_f64(run_data.y_led, 1_000_000),
            );
    
            // Update the last known position of the driver
            self.last_positions.insert(run_data.driver_number, coord_key);
        }
        
        // Debug print for last positions
        println!("Last positions: {:?}", self.last_positions);
        
        // Update the LED states for all known positions
        for (&driver_number, &position) in &self.last_positions {
            let color = self
                .driver_info
                .iter()
                .find(|&driver| driver.number == driver_number)
                .map_or(egui::Color32::WHITE, |driver| driver.color);
    
            // Print LED positions and corresponding colors
            println!("LED position: {:?}, Color: {:?}", position, color);
    
            self.led_states.insert(position, color);
        }
    
        // Debug print for LED states
        println!("LED states: {:?}", self.led_states);
    }
    
    async fn visualize_data(&mut self, run_race_data: Vec<RunRace>) {
        println!("Visualizing data...");
        self.update_with_data(run_race_data);
    
        // Debug print for run_race_data
        //println!("Run race data: {:?}", self.run_race_data);
    
        self.update_led_states();
    }

    fn scale_f64(value: f64, scale: i64) -> i64 {
        (value * scale as f64) as i64
    }

    fn update_with_data(&mut self, data: Vec<RunRace>) {
        self.run_race_data.extend(data);
        self.run_race_data.sort_by_key(|d| std::cmp::Reverse(d.date));
    }

    async fn load_data(&mut self) -> Result<(), Box<dyn StdError + Send + Sync>> {
        let driver_numbers = vec![
            1, 2, 4, 10, 11, 14, 16, 18, 20, 22, 23, 24, 27, 31, 40, 44, 55, 63, 77, 81,
        ];
    
        let mut handles = Vec::new();
        let max_rows_per_driver = 500;
    
        for driver_number in driver_numbers {
            let url = format!(
                "https://api.openf1.org/v1/location?session_key={}&driver_number={}",
                "9149", driver_number
            );
    
            let mut app_clone = self.clone();
            handles.push(tokio::spawn(async move {
                let mut stream = fetch_data_in_chunks(&url, 8 * 1048).await?;
                let mut buffer = Vec::new();
    
                while let Some(chunk) = stream.next().await {
                    let chunk = chunk?;
                    println!("Received a chunk of data for driver number {}", driver_number);
                    let run_race_data = process_chunk(chunk, &mut buffer, &app_clone.coordinates, max_rows_per_driver).await?;
                    app_clone.visualize_data(run_race_data).await;  // Call visualize_data directly
    
                    if buffer.len() >= max_rows_per_driver {
                        break;
                    }
                }
    
                Ok::<(), Box<dyn StdError + Send + Sync>>(())
            }));
        }
    
        // Await all tasks concurrently using `tokio::join!`
        let results = futures::future::join_all(handles).await;
    
        // Handle any errors
        for result in results {
            if let Err(e) = result {
                eprintln!("Error fetching data: {:?}", e);
            }
        }
    
        println!("Finished streaming data for all drivers");
        self.data_loaded = true; // Set the flag to true
    
        if let Some(sender) = &self.completion_sender {
            let _ = sender.send(()).await;
        }
    
        Ok(())
    }
}


impl App for PlotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.update_race();

        // Poll the channel for completion messages
        if let Some(receiver) = self.completion_receiver.as_ref() {
            if let Ok(()) = receiver.try_recv() {
                ctx.request_repaint(); // Force repaint after data is loaded
            }
        }

        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Background,
            egui::Id::new("layer"),
        ));

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
                    (self.race_time / 3600.0).floor() as u32,
                    ((self.race_time % 3600.0) / 60.0).floor() as u32,
                    self.race_time % 60.0
                ));
                ui.separator();

                if ui.button("START").clicked() {
                    if !self.data_loading_started {
                        self.data_loading_started = true;
                        self.race_started = true;
                        self.start_time = Instant::now();
                        let mut app_clone = self.clone();
                        let sender = self.completion_sender.clone().unwrap();
                        tokio::spawn(async move {
                            app_clone.load_data().await.unwrap();
                            let _ = sender.send(()).await; // Notify completion
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
                style.text_styles.get_mut(&egui::TextStyle::Body).unwrap().size = 8.0;

                for driver in &self.driver_info {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}: {} ({})", driver.number, driver.name, driver.team));
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
                let norm_y = (ui.available_height() - 60.0) - (((coord.y_led - min_y) / height) as f32 * (ui.available_height() - 60.0));

                painter.rect_filled(
                    egui::Rect::from_min_size(egui::pos2(norm_x + 30.0, norm_y + 30.0), egui::vec2(20.0, 20.0)),
                    egui::Rounding::same(0.0),
                    egui::Color32::BLACK,
                );
            }

            for ((x, y), color) in &self.led_states {
                let norm_x = ((*x as f64 / 1_000_000.0 - min_x) / width) as f32 * (ui.available_width() - 60.0);
                let norm_y = (ui.available_height() - 60.0) - (((*y as f64 / 1_000_000.0 - min_y) / height) as f32 * (ui.available_height() - 60.0));

                painter.rect_filled(
                    egui::Rect::from_min_size(egui::pos2(norm_x + 30.0, norm_y + 30.0), egui::vec2(20.0, 20.0)),
                    egui::Rounding::same(0.0),
                    *color,
                );
            }
        });

        ctx.request_repaint();
    }
}


fn read_coordinates() -> Result<Vec<LedCoordinate>, Box<dyn StdError>> {
    Ok(vec![
        LedCoordinate { x_led: 6413.0, y_led: 33.0 },
        LedCoordinate { x_led: 6007.0, y_led: 197.0 },
        LedCoordinate { x_led: 5652.0, y_led: 444.0 },
        LedCoordinate { x_led: 5431.0, y_led: 822.0 },
        LedCoordinate { x_led: 5727.0, y_led: 1143.0 },
        LedCoordinate { x_led: 6141.0, y_led: 1268.0 },
        LedCoordinate { x_led: 6567.0, y_led: 1355.0 },
        LedCoordinate { x_led: 6975.0, y_led: 1482.0 },
        LedCoordinate { x_led: 7328.0, y_led: 1738.0 },
        LedCoordinate { x_led: 7369.0, y_led: 2173.0 },
        LedCoordinate { x_led: 7024.0, y_led: 2448.0 },
        LedCoordinate { x_led: 6592.0, y_led: 2505.0 },
        LedCoordinate { x_led: 6159.0, y_led: 2530.0 },
        LedCoordinate { x_led: 5725.0, y_led: 2525.0 },
        LedCoordinate { x_led: 5288.0, y_led: 2489.0 },
        LedCoordinate { x_led: 4857.0, y_led: 2434.0 },
        LedCoordinate { x_led: 4429.0, y_led: 2356.0 },
        LedCoordinate { x_led: 4004.0, y_led: 2249.0 },
        LedCoordinate { x_led: 3592.0, y_led: 2122.0 },
        LedCoordinate { x_led: 3181.0, y_led: 1977.0 },
        LedCoordinate { x_led: 2779.0, y_led: 1812.0 },
        LedCoordinate { x_led: 2387.0, y_led: 1624.0 },
        LedCoordinate { x_led: 1988.0, y_led: 1453.0 },
        LedCoordinate { x_led: 1703.0, y_led: 1779.0 },
        LedCoordinate { x_led: 1271.0, y_led: 1738.0 },
        LedCoordinate { x_led: 1189.0, y_led: 1314.0 },
        LedCoordinate { x_led: 1257.0, y_led: 884.0 },
        LedCoordinate { x_led: 1333.0, y_led: 454.0 },
        LedCoordinate { x_led: 1409.0, y_led: 25.0 },
        LedCoordinate { x_led: 1485.0, y_led: -405.0 },
        LedCoordinate { x_led: 1558.0, y_led: -835.0 },
        LedCoordinate { x_led: 1537.0, y_led: -1267.0 },
        LedCoordinate { x_led: 1208.0, y_led: -1555.0 },
        LedCoordinate { x_led: 779.0, y_led: -1606.0 },
        LedCoordinate { x_led: 344.0, y_led: -1604.0 },
        LedCoordinate { x_led: -88.0, y_led: -1539.0 },
        LedCoordinate { x_led: -482.0, y_led: -1346.0 },
        LedCoordinate { x_led: -785.0, y_led: -1038.0 },
        LedCoordinate { x_led: -966.0, y_led: -644.0 },
        LedCoordinate { x_led: -1015.0, y_led: -206.0 },
        LedCoordinate { x_led: -923.0, y_led: 231.0 },
        LedCoordinate { x_led: -762.0, y_led: 650.0 },
        LedCoordinate { x_led: -591.0, y_led: 1078.0 },
        LedCoordinate { x_led: -423.0, y_led: 1497.0 },
        LedCoordinate { x_led: -254.0, y_led: 1915.0 },
        LedCoordinate { x_led: -86.0, y_led: 2329.0 },
        LedCoordinate { x_led: 83.0, y_led: 2744.0 },
        LedCoordinate { x_led: 251.0, y_led: 3158.0 },
        LedCoordinate { x_led: 416.0, y_led: 3574.0 },
        LedCoordinate { x_led: 588.0, y_led: 3990.0 },
        LedCoordinate { x_led: 755.0, y_led: 4396.0 },
        LedCoordinate { x_led: 920.0, y_led: 4804.0 },
        LedCoordinate { x_led: 1086.0, y_led: 5212.0 },
        LedCoordinate { x_led: 1250.0, y_led: 5615.0 },
        LedCoordinate { x_led: 1418.0, y_led: 6017.0 },
        LedCoordinate { x_led: 1583.0, y_led: 6419.0 },
        LedCoordinate { x_led: 1909.0, y_led: 6702.0 },
        LedCoordinate { x_led: 2306.0, y_led: 6512.0 },
        LedCoordinate { x_led: 2319.0, y_led: 6071.0 },
        LedCoordinate { x_led: 2152.0, y_led: 5660.0 },
        LedCoordinate { x_led: 1988.0, y_led: 5255.0 },
        LedCoordinate { x_led: 1853.0, y_led: 4836.0 },
        LedCoordinate { x_led: 1784.0, y_led: 4407.0 },
        LedCoordinate { x_led: 1779.0, y_led: 3971.0 },
        LedCoordinate { x_led: 1605.0, y_led: 3569.0 },
        LedCoordinate { x_led: 1211.0, y_led: 3375.0 },
        LedCoordinate { x_led: 811.0, y_led: 3188.0 },
        LedCoordinate { x_led: 710.0, y_led: 2755.0 },
        LedCoordinate { x_led: 1116.0, y_led: 2595.0 },
        LedCoordinate { x_led: 1529.0, y_led: 2717.0 },
        LedCoordinate { x_led: 1947.0, y_led: 2848.0 },
        LedCoordinate { x_led: 2371.0, y_led: 2946.0 },
        LedCoordinate { x_led: 2806.0, y_led: 2989.0 },
        LedCoordinate { x_led: 3239.0, y_led: 2946.0 },
        LedCoordinate { x_led: 3665.0, y_led: 2864.0 },
        LedCoordinate { x_led: 4092.0, y_led: 2791.0 },
        LedCoordinate { x_led: 4523.0, y_led: 2772.0 },
        LedCoordinate { x_led: 4945.0, y_led: 2886.0 },
        LedCoordinate { x_led: 5331.0, y_led: 3087.0 },
        LedCoordinate { x_led: 5703.0, y_led: 3315.0 },
        LedCoordinate { x_led: 6105.0, y_led: 3484.0 },
        LedCoordinate { x_led: 6538.0, y_led: 3545.0 },
        LedCoordinate { x_led: 6969.0, y_led: 3536.0 },
        LedCoordinate { x_led: 7402.0, y_led: 3511.0 },
        LedCoordinate { x_led: 7831.0, y_led: 3476.0 },
        LedCoordinate { x_led: 8241.0, y_led: 3335.0 },
        LedCoordinate { x_led: 8549.0, y_led: 3025.0 },
        LedCoordinate { x_led: 8703.0, y_led: 2612.0 },
        LedCoordinate { x_led: 8662.0, y_led: 2173.0 },
        LedCoordinate { x_led: 8451.0, y_led: 1785.0 },
        LedCoordinate { x_led: 8203.0, y_led: 1426.0 },
        LedCoordinate { x_led: 7973.0, y_led: 1053.0 },
        LedCoordinate { x_led: 7777.0, y_led: 664.0 },
        LedCoordinate { x_led: 7581.0, y_led: 275.0 },
        LedCoordinate { x_led: 7274.0, y_led: -35.0 },
        LedCoordinate { x_led: 6839.0, y_led: -46.0 },
    ])
}

fn generate_run_race_data(
    raw_data: &[LocationData],
    coordinates: &[LedCoordinate],
) -> Vec<RunRace> {
    raw_data
        .iter()
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

            RunRace {
                date: data.date,
                driver_number: data.driver_number,
                x_led: nearest_coord.x_led,
                y_led: nearest_coord.y_led,
            }
        })
        .collect()
}

async fn fetch_data_in_chunks(url: &str, _chunk_size: usize) -> Result<impl futures_util::stream::Stream<Item = Result<Bytes, reqwest::Error>>, Box<dyn StdError + Send + Sync>> {
    let client = Client::new();
    let resp = client.get(url).send().await?.error_for_status()?;
    let stream = resp.bytes_stream();
    Ok(stream)
}


async fn process_chunk(
    chunk: Bytes,
    buffer: &mut Vec<u8>,
    coordinates: &[LedCoordinate],
    max_rows: usize
) -> Result<Vec<RunRace>, Box<dyn StdError + Send + Sync>> {
    buffer.extend_from_slice(&chunk);
    println!("Processing a new chunk of data...");
    println!("Current buffer content size: {}", buffer.len());

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

        println!("Attempting to deserialize slice: {}", json_slice);

        match serde_json::from_str::<LocationData>(&json_slice) {
            Ok(location_data) => {
                println!("Deserialized JSON data: {:?}", location_data);

                let new_run_race_data = generate_run_race_data(&[location_data], coordinates);

                rows_processed += new_run_race_data.len();
                run_race_data.extend(new_run_race_data);
                start_pos += end_pos + 3; // Move past the processed part
                if rows_processed >= max_rows {
                    println!("Reached max rows limit: {}", max_rows);
                    break;
                }
            }
            Err(e) => {
                println!("Failed to deserialize LocationData: {:?}", e);
                break; // Break the loop if we can't deserialize a complete object
            }
        }
    }

    // Retain the unprocessed part of the buffer
    *buffer = buffer_str[start_pos..].as_bytes().to_vec();

    // Check if the remaining buffer contains a complete JSON object and process it
    if let Ok(location_data) = serde_json::from_slice::<LocationData>(&buffer) {
        let new_run_race_data = generate_run_race_data(&[location_data], coordinates);
        run_race_data.extend(new_run_race_data);
        *buffer = Vec::new(); // Clear the buffer after processing
    }

    Ok(run_race_data)
}



/* 
async fn visualize_data(&mut self, run_race_data: Vec<RunRace>) {
    println!("Visualizing data...");
    self.update_with_data(run_race_data);

    // Debug print for run_race_data
    println!("Run race data: {:?}", self.run_race_data);

    self.update_led_states();
}
*/

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

    let app = PlotApp::new(coordinates, driver_info);

    let native_options = eframe::NativeOptions::default();
    eframe::run_native("F1-LED-CIRCUIT SIMULATION", native_options, Box::new(|_cc| Box::new(app)))?;

    Ok(())
}
