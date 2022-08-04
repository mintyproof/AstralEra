use eframe::egui;
use super::launcher::*;

pub struct LauncherApp {
    textfield_username: String,
    textfield_password: String,
    remember_login: bool,
    logged_in: bool,
    user_session: String
}

impl Default for LauncherApp {
    fn default() -> Self {
        Self {
            textfield_username: "".to_owned(),
            textfield_password: "".to_owned(),
            remember_login: false,
            logged_in: false,
            user_session: "".to_owned()
        }
    }
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("AstralEra");

            if self.logged_in {
                ui.label(format!("Logged in as {}", self.textfield_username));
                ui.horizontal(|ui| {
                    ui.label("Not you?");
                    if ui.button("Sign out").clicked() {
                        self.textfield_username = "".to_owned();
                        self.textfield_password = "".to_owned();
                        self.remember_login = false;
                        self.logged_in = false;
                    }
                });

                if ui.button("Launch game").clicked() {
                    launch_game(GAME_EXE_GAME, GAME_PATH);
                }
            } else {
                ui.horizontal(|ui| {
                    ui.label("Username ");
                    ui.text_edit_singleline(&mut self.textfield_username);
                });
                ui.horizontal(|ui| {
                    ui.label("Password ");
                    ui.text_edit_singleline(&mut self.textfield_password);
                });
                ui.checkbox(&mut self.remember_login, "Remember. Remember me. Remember that I once logged in");
                
                if ui.button("Log in").clicked() {
                    self.logged_in = true;
                }
            }
        });
    }
}