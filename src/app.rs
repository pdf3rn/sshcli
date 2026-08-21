use crate::profiles::Profile;

#[derive(Clone)]
pub enum Action {
    Connect(Profile),
    Sftp(Profile),
}

pub struct App {
    pub should_quit: bool,
    pub selected_profile: usize,
    pub status: String,
    pub profiles: Vec<Profile>,
}

impl Default for App {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl App {
    pub fn new(profiles: Vec<Profile>) -> Self {
        Self {
            should_quit: false,
            selected_profile: 0,
            status: if profiles.is_empty() {
                "No profiles. Use: sshcli profile add <name>".into()
            } else {
                "Select a profile and press Enter to connect.".into()
            },
            profiles,
        }
    }

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

    pub fn selected_profile(&self) -> Option<Profile> {
        self.profiles.get(self.selected_profile).cloned()
    }
}
