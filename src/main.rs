use dexrs::dexcom::client::DexcomClient;
use dirs::config_dir;
use freya::{
    prelude::*,
    winit::dpi::{LogicalPosition, LogicalSize},
};
use futures_timer::Delay;
use keyring::Entry;
use std::{fs, path::PathBuf, sync::LazyLock, time::Duration};

static USERNAME_PATH: LazyLock<PathBuf> = LazyLock::new(|| username_path());

enum AppState {
    Login,
    Main,
}

fn main() {
    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new(app)
                .with_title("Glyx")
                .with_transparency(true)
                .with_background(Color::TRANSPARENT),
        ),
    );
}

fn app() -> impl IntoElement {
    let entry = use_state(|| Entry::new("glyx", "password").unwrap());

    let platform = Platform::get();
    let mut state = use_state(|| AppState::Login);

    let mut client: State<Option<DexcomClient>> = use_state(|| match entry.read().get_password() {
        Ok(password) => match get_name() {
            Some(username) => match DexcomClient::new(username, password, true) {
                Ok(c) => {
                    state.set(AppState::Main);
                    Some(c)
                }
                Err(_) => None,
            },
            None => None,
        },
        Err(_) => None,
    });

    let username_input = use_state(|| String::new());
    let password_input = use_state(|| String::new());

    match &*state.read() {
        AppState::Login => {
            platform.with_window(None, |window| window.set_cursor_hittest(true).unwrap());
            let login = rect()
                .vertical()
                .center()
                .spacing(5.)
                .width(Size::percent(100.))
                .height(Size::percent(95.))
                .child(
                    Input::new(username_input)
                        .placeholder("Username")
                        .width(Size::percent(50.)),
                )
                .child(
                    Input::new(password_input)
                        .placeholder("Password")
                        .width(Size::percent(50.))
                        .mode(InputMode::Hidden('*')),
                )
                .child(Button::new().child("Login").on_press(move |_| {
                    match DexcomClient::new(
                        username_input.read().clone(),
                        password_input.read().clone(),
                        true,
                    ) {
                        Ok(c) => {
                            client.set(Some(c));
                            entry
                                .read()
                                .set_password(password_input.read().as_str())
                                .unwrap();
                            create_username(&*username_input.read());
                            state.set(AppState::Main);
                        }
                        Err(_) => {}
                    }
                }));

            login.background(Color::WHITE).expanded()
        }
        AppState::Main => {
            platform.with_window(None, |window| {
                window.set_cursor_hittest(false).unwrap();
                window.set_decorations(false);
                window.set_window_level(freya::winit::window::WindowLevel::AlwaysOnTop);
                window.set_outer_position(LogicalPosition::new(10., 10.));
                let _ = window.request_inner_size(LogicalSize::new(120., 50.));
            });

            let mut reading = use_state(|| String::new());

            use_future(move || async move {
                loop {
                    reading.set(glucose(client.read().as_ref().unwrap()));

                    Delay::new(Duration::from_secs(3)).await;
                }
            });

            rect()
                .child(
                    rect()
                        .child(format!("{}", reading.read()))
                        .center()
                        .font_weight(FontWeight::BOLD)
                        .background(Color::from_argb(160, 255, 255, 255))
                        .width(Size::px(120.))
                        .corner_radius(10.),
                )
                .color(Color::BLACK)
                .text_align(TextAlign::Start)
                .font_size(30.)
        }
    }
}

fn get_name() -> Option<String> {
    match fs::read_to_string(&*USERNAME_PATH) {
        Ok(username) => Some(username),
        Err(_) => None,
    }
}

fn username_path() -> PathBuf {
    let mut path = config_dir().unwrap();
    path.push("dexcom_username");
    path
}

fn create_username(name: impl Into<String>) {
    fs::write(&*USERNAME_PATH, name.into()).unwrap();
}

fn glucose(client: &DexcomClient) -> String {
    match client.get_glucose_readings(None, None) {
        Ok(g) => {
            let reading = g.last().unwrap();
            format!("{}{}", reading.mg_dl, reading.trend.arrow)
        }
        Err(_) => String::from("---"),
    }
}
