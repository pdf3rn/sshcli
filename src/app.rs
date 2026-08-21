use crate::profiles::{Authentication, Profile};

pub struct ProfileDraft {
    pub name: String,
    pub host: String,
    pub port: String,
    pub username: String,
    pub identity_file: String,
    pub authentication: Authentication,
    pub accept_unknown_host_key: bool,
}

pub enum Action {
    Connect(Profile),
    Sftp(Profile),
    Forward(Profile),
    Delete(Profile),
    CloseSession(Profile),
    Create(ProfileDraft),
}

pub struct App {
    pub should_quit: bool,
    pub selected_profile: usize,
    pub status: String,
    pub profiles: Vec<Profile>,
    pub active_sessions: Vec<String>,
    pub delete_confirmation: Option<String>,
}

impl Default for App {
    fn default() -> Self {
        Self::new(Vec::new(), &[])
    }
}

impl App {
    pub fn new(profiles: Vec<Profile>, active_sessions: &[String]) -> Self {
        Self {
            should_quit: false,
            selected_profile: 0,
            status: if profiles.is_empty() {
                "No profiles. Press n to create one.".into()
            } else {
                "Select a profile and press Enter to connect.".into()
            },
            profiles,
            active_sessions: active_sessions.to_vec(),
            delete_confirmation: None,
        }
    }

    pub fn is_active(&self, name: &str) -> bool {
        self.active_sessions.iter().any(|session| session == name)
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn select_next(&mut self) {
        self.delete_confirmation = None;
        if !self.profiles.is_empty() {
            self.selected_profile = (self.selected_profile + 1) % self.profiles.len();
        }
    }

    pub fn select_previous(&mut self) {
        self.delete_confirmation = None;
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
