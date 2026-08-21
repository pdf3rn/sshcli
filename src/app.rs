pub struct App {
    pub should_quit: bool,
    pub selected_profile: usize,
    pub status: String,
    pub profiles: Vec<String>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            should_quit: false,
            selected_profile: 0,
            status: "No profiles yet. Press n to add one in a future release.".into(),
            profiles: vec!["No saved connections".into()],
        }
    }
}

impl App {
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn select_next(&mut self) {
        if !self.profiles.is_empty() {
            self.selected_profile = (self.selected_profile + 1) % self.profiles.len();
        }
    }

    pub fn select_previous(&mut self) {
        if !self.profiles.is_empty() {
            self.selected_profile = self
                .selected_profile
                .checked_sub(1)
                .unwrap_or(self.profiles.len() - 1);
        }
    }
}
