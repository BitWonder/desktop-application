// plotters-iced
//
// Iced backend for Plotters
// Copyright: 2022, Joylei <leingliu@gmail.com>
// License: MIT
use chrono::{DateTime, Utc};
use iced::{
    font, futures::lock::Mutex, widget::{
        button, canvas::{Cache, Frame, Geometry}, Column, Container, Text
    }, Alignment, Element, Font, Length, Size, Subscription, Task
};
use plotters::prelude::ChartBuilder;
use plotters_backend::DrawingBackend;
use plotters_iced::{
    sample::lttb::{DataPoint, LttbSource},
    Chart, ChartWidget, Renderer,
};
use std::{fs::File, io::Read, sync::Arc, time::Duration};
use std::{collections::VecDeque, time::Instant};
use std::{time::UNIX_EPOCH, io::Write};

use serialport::StopBits;

const TITLE_FONT_SIZE: u16 = 22;

const FONT_BOLD: Font = Font {
    family: font::Family::Name("Noto Sans"),
    weight: font::Weight::Bold,
    ..Font::DEFAULT
};

#[tokio::main]
async fn main() {
    let link: Arc<Mutex<Vec<(DateTime<Utc>, f32)>>> = Arc::new(Mutex::new(vec![(chrono::DateTime::from_timestamp_millis(std::time::SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64).unwrap(), 0.0)]));
    let state_link = Arc::clone(&link);

    tokio::spawn(generate_new_data(link));

    iced::application("Large Data Example", State::update, State::view)
        .subscription(subscription)
        .antialiasing(true)
        .default_font(Font::with_name("Noto Sans"))
        .run_with(|| State::new(state_link))
        .unwrap();
}

fn subscription(_state: &State) -> Subscription<Message> {
    iced::time::every(Duration::from_secs(1)).map(move |_| {
        Message::ReloadData
    })
}


struct Wrapper<'a>(&'a DateTime<Utc>, &'a f32);

impl DataPoint for Wrapper<'_> {
    #[inline]
    fn x(&self) -> f64 {
        self.0.timestamp() as f64
    }
    #[inline]
    fn y(&self) -> f64 {
        *self.1 as f64
    }
}

#[derive(Debug, Clone)]
enum Message {
    FontLoaded(Result<(), font::Error>),
    DataLoaded(Vec<(DateTime<Utc>, f32)>),
    Sampled(Vec<(DateTime<Utc>, f32)>),
    ReloadData
}

struct State {
    chart: Option<ExampleChart>,
    data: Arc<Mutex<Vec<(DateTime<Utc>, f32)>>>
}

impl State {
    fn new(data: Arc<Mutex<Vec<(DateTime<Utc>, f32)>>>) -> (Self, Task<Message>) {
        (
            Self { chart: None, data: data.clone() },
            Task::batch([
                font::load(include_bytes!("./fonts/notosans-regular.ttf").as_slice())
                    .map(Message::FontLoaded),
                font::load(include_bytes!("./fonts/notosans-bold.ttf").as_slice())
                    .map(Message::FontLoaded),
                Task::perform(tokio::task::spawn(generate_data(data.clone())), |data| {
                    Message::DataLoaded(data.unwrap())
                }),
            ]),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::DataLoaded(data) => Task::perform(
                tokio::task::spawn_blocking(move || {
                    let now = Instant::now();
                    let sampled: Vec<_> = (&data[..])
                        .cast(|v| Wrapper(&v.0, &v.1))
                        .lttb(1000)
                        .map(|w| (*w.0, *w.1))
                        .collect();
                    dbg!(now.elapsed().as_millis());
                    sampled
                }),
                |data| Message::Sampled(data.unwrap()),
            ),
            Message::Sampled(sampled) => {
                self.chart = Some(ExampleChart::new(sampled.into_iter()));
                Task::none()
            },
            Message::ReloadData => {
                Task::perform(tokio::task::spawn(generate_data(self.data.clone())), |data| {
                    Message::DataLoaded(data.unwrap())
                })
            }
            _ => Task::none(),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let content = Column::new()
            .spacing(20)
            .align_x(Alignment::Start)
            .width(Length::Fill)
            .height(Length::Fill)
            .push( // header
                Text::new("Iced test chart")
                    .size(TITLE_FONT_SIZE)
                    .font(FONT_BOLD),
            )
            .push(match self.chart { // chart
                Some(ref chart) => chart.view(),
                None => Text::new("Loading...").into(),
            })
            .push(button("Reload Chart").on_press(Message::ReloadData));

        Container::new(content)
            .padding(5)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}

struct ExampleChart {
    cache: Cache,
    data_points: VecDeque<(DateTime<Utc>, f32)>,
}

impl ExampleChart {
    fn new(data: impl Iterator<Item = (DateTime<Utc>, f32)>) -> Self {
        let data_points: VecDeque<_> = data.collect();
        Self {
            cache: Cache::new(),
            data_points,
        }
    }

    fn view(&self) -> Element<Message> {
        let chart = ChartWidget::new(self)
            .width(Length::Fill)
            .height(Length::Fill);

        chart.into()
    }
}

impl Chart<Message> for ExampleChart {
    type State = ();
    // fn update(
    //     &mut self,
    //     event: Event,
    //     bounds: Rectangle,
    //     cursor: Cursor,
    // ) -> (event::Status, Option<Message>) {
    //     self.cache.clear();
    //     (event::Status::Ignored, None)
    // }

    #[inline]
    fn draw<R: Renderer, F: Fn(&mut Frame)>(
        &self,
        renderer: &R,
        bounds: Size,
        draw_fn: F,
    ) -> Geometry {
        renderer.draw_cache(&self.cache, bounds, draw_fn)
    }

    fn build_chart<DB: DrawingBackend>(&self, _state: &Self::State, mut chart: ChartBuilder<DB>) {
        use plotters::prelude::*;

        const PLOT_LINE_COLOR: RGBColor = RGBColor(0, 175, 255);

        // Acquire time range
        let newest_time = self
            .data_points
            .back()
            .unwrap()
            .0
            .checked_add_signed(chrono::Duration::from_std(Duration::from_secs(10)).unwrap())
            .unwrap();
        //let oldest_time = newest_time - chrono::Duration::seconds(PLOT_SECONDS as i64);
        let oldest_time = self
            .data_points
            .front()
            .unwrap()
            .0
            .checked_sub_signed(chrono::Duration::from_std(Duration::from_secs(10)).unwrap())
            .unwrap();
        //dbg!(&newest_time);
        //dbg!(&oldest_time);
        let mut chart = chart
            .x_label_area_size(20)
            .y_label_area_size(80)
            .margin(20)
            .build_cartesian_2d(oldest_time..newest_time, self.data_points.clone().into_iter().min_by_key(|x| x.1 as u64).unwrap().1 as f32..self.data_points.clone().into_iter().max_by_key(|x| x.1 as u64).unwrap().1 as f32)
            .expect("failed to build chart");

        chart
            .configure_mesh()
            .bold_line_style(plotters::style::colors::BLUE.mix(0.1))
            .light_line_style(plotters::style::colors::BLUE.mix(0.05))
            .axis_style(ShapeStyle::from(plotters::style::colors::BLUE.mix(0.45)).stroke_width(1))
            .y_labels(10)
            .y_label_style(
                ("Noto Sans", 15)
                    .into_font()
                    .color(&plotters::style::colors::BLUE.mix(0.65))
                    .transform(FontTransform::Rotate90),
            )
            .y_label_formatter(&|y| format!("{}", y))
            .x_labels(5)
            .x_label_style(
                ("Noto Sans", 15)
                    .into_font()
                    .color(&plotters::style::colors::BLUE.mix(0.65))
                    .transform(FontTransform::Rotate90),
            )
            .x_label_formatter(&|y| format!("{}", y))
            .draw()
            .expect("failed to draw chart mesh");

        chart
            .draw_series(
                AreaSeries::new(
                    self.data_points.iter().cloned(),
                    0_f32,
                    PLOT_LINE_COLOR.mix(0.175),
                )
                .border_style(ShapeStyle::from(PLOT_LINE_COLOR).stroke_width(2)),
            )
            .expect("failed to draw chart data");
    }
}

async fn generate_data(save_data: Arc<Mutex<Vec<(DateTime<Utc>, f32)>>>) -> Vec<(DateTime<Utc>, f32)> {
    /*let mut f: File = File::open("./graph_testing_data/value_of_bpm_at_1729445874626.csv").unwrap();
    let mut buffer: String = String::new();
    f.read_to_string(&mut buffer).unwrap();
    let data: Vec<(DateTime<Utc>, f32)> = buffer.split("\n").map(|x| {
        let a: Vec<u64> = x.split(",").map(|x| x.trim().parse::<u64>().unwrap_or(1)).collect();
        let t: DateTime<Utc> = DateTime::<Utc>::from_timestamp_millis(a[0] as i64).unwrap();
        let p: f32;
        if a.len() <= 1 {
            p = 100.0;
        } else { // this will be last line so be curr time
            p = a[1] as f32;
        }
        (t, p)
    }).collect();
    return data;*/
    let data = save_data.lock().await;
    return data.to_vec();
    /*let total = 10_000_000;
    let mut data = Vec::new();
    let mut rng = rand::thread_rng();
    let time_range = (24 * 3600 * 30) as f32;
    let interval = (3600 * 12) as f32;
    let start = Utc::now()
        .checked_sub_signed(
            chrono::Duration::from_std(Duration::from_secs_f32(time_range)).unwrap(),
        )
        .unwrap();
    while data.len() < total {
        let secs = rng.gen_range(0.1..time_range);
        let time = start
            .checked_sub_signed(chrono::Duration::from_std(Duration::from_secs_f32(secs)).unwrap())
            .unwrap();

        let value =
            (((secs % interval) - interval / 2.0) / (interval / 2.0) * std::f32::consts::PI).sin()
                * 50_f32
                + 50_f32;
        data.push((time, value));
    }
    data.sort_by_cached_key(|x| x.0);
    //dbg!(&data[..100]);
    data*/
}

async fn generate_new_data(save_data: Arc<Mutex<Vec<(DateTime<Utc>, f32)>>>) {
    let mut f: File = File::create("./graph_testing_data/value_of_bpm_at_1729445874626.csv").unwrap();
    let ports = serialport::available_ports().expect("No ports found!");
    for p in ports.clone() {
        println!("{}", p.port_name);
    }

    if ports[0].port_name.clone() != "/dev/ttyACM0" {
        return;
    }

    let mut port = serialport::new(ports[0].port_name.clone(), 9600)
        .stop_bits(StopBits::One)
        .parity(serialport::Parity::Even)
        .timeout(std::time::Duration::from_millis(1000)) // wait one full second for data to flow
        .open().expect("Failed to open port");

    dbg!(port.try_clone().unwrap());

    let _ = port.write(b"\r\n");
    let mut x: usize = 0;
    let mut pulse_value: Vec<char> = Vec::new();
    let mut current_time: u64 = 0;
    loop {
        // let _ = port.write(format!("Hello from Rust: serialport try: {x}!\r").as_bytes());
        let mut serial_buf: [u8; 1] = [0];
        let _ = port.read_exact(&mut serial_buf);
        //(0..3).into_iter().for_each(|_|{  serial_buf.pop(); });
        if serial_buf[0].is_ascii_digit() {
            pulse_value.push(char::from_u32(serial_buf[0] as u32).unwrap());
            if current_time == 0 {
                current_time = std::time::SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
            }
        }
        else if pulse_value.len() > 0 {
            let intermediatery_value: String = pulse_value.iter().collect::<String>();
            let value = intermediatery_value.parse::<usize>().unwrap();
            writeln!(f, "{},{}", current_time,value).unwrap(); // cur time x and pulse val y
            // send to linked data
            let mut data = save_data.lock().await;
            data.push((DateTime::from_timestamp_millis(current_time as i64).unwrap(), value as f32));
            pulse_value.clear(); // remove data to make way for new data
            current_time = 0;
        }
        else {
            x = x.wrapping_add(1);
            //println!("{x}");
        }
    }
}