//! Page zoom: the levels, the commands that move between them and the saved level. The macOS View
//! menu that drives it is in `zoom_menu`; WebView2 gets its zoom keys from the window builder.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// The zoom levels in percent, as browsers offer them. The saved file holds one of these numbers,
/// so changing a value forgets a saved level and nothing else.
const PERCENTS: [u16; 12] = [50, 67, 80, 90, 100, 110, 125, 150, 175, 200, 250, 300];

/// Where `PERCENTS` holds 100.
const ACTUAL: usize = 4;

/// The file in the app's config folder that remembers the level.
pub(crate) const SAVED: &str = "zoom";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Command {
    In,
    Out,
    Actual,
}

impl Command {
    /// The index this command leads to from `index`, stopping at either end.
    fn apply(self, index: usize) -> usize {
        match self {
            Self::In => index.saturating_add(1).min(PERCENTS.len() - 1),
            Self::Out => index.saturating_sub(1),
            Self::Actual => ACTUAL,
        }
    }
}

/// One menu row. Zoom In has two: `=` is where `+` sits on a US keyboard, and a layout where `+` is
/// a key of its own sends the other.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Shortcut {
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) accelerator: &'static str,
    pub(crate) command: Command,
}

pub(crate) const SHORTCUTS: [Shortcut; 4] = [
    Shortcut {
        id: "zoom-in",
        label: "Zoom In",
        accelerator: "CmdOrCtrl++",
        command: Command::In,
    },
    Shortcut {
        id: "zoom-in-equals",
        label: "Zoom In",
        accelerator: "CmdOrCtrl+=",
        command: Command::In,
    },
    Shortcut {
        id: "zoom-out",
        label: "Zoom Out",
        accelerator: "CmdOrCtrl+-",
        command: Command::Out,
    },
    Shortcut {
        id: "zoom-actual",
        label: "Actual Size",
        accelerator: "CmdOrCtrl+0",
        command: Command::Actual,
    },
];

/// The command a menu id stands for, if it is one of ours.
pub(crate) fn command_for(id: &str) -> Option<Command> {
    SHORTCUTS
        .iter()
        .find(|shortcut| shortcut.id == id)
        .map(|shortcut| shortcut.command)
}

fn percent(index: usize) -> u16 {
    PERCENTS.get(index).copied().unwrap_or(100)
}

/// The index for a saved file's text; anything that is not a known level is Actual Size.
fn parse_saved(text: &str) -> usize {
    text.trim()
        .parse::<u16>()
        .ok()
        .and_then(|saved| PERCENTS.iter().position(|level| *level == saved))
        .unwrap_or(ACTUAL)
}

/// The current level and where it is kept. The level lives in memory, so a save that fails costs
/// only the next start, never the zoom the person is looking at.
#[derive(Debug)]
pub(crate) struct Zoom {
    index: usize,
    saved: Option<PathBuf>,
}

impl Zoom {
    /// Starts from the level saved at `saved`; a missing or unreadable file, the first run, is
    /// Actual Size.
    pub(crate) fn load(saved: Option<PathBuf>) -> Self {
        let index = saved
            .as_deref()
            .and_then(|path| fs::read_to_string(path).ok())
            .map_or(ACTUAL, |text| parse_saved(&text));
        Self { index, saved }
    }

    /// The page zoom factor for the current level.
    pub(crate) fn factor(&self) -> f64 {
        f64::from(percent(self.index)) / 100.0
    }

    pub(crate) fn is_actual(&self) -> bool {
        self.index == ACTUAL
    }

    /// Moves to the level `command` leads to, keeps it in memory, and tries to save it.
    ///
    /// # Errors
    ///
    /// Returns the write error when the level could not be saved; the new level still holds.
    pub(crate) fn step(&mut self, command: Command) -> io::Result<()> {
        self.index = command.apply(self.index);
        match &self.saved {
            Some(path) => save(path, self.index),
            None => Ok(()),
        }
    }
}

fn save(path: &Path, index: usize) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, percent(index).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("archee-zoom-{name}-{}", std::process::id()))
    }

    fn percent_of(zoom: &Zoom) -> u16 {
        percent(zoom.index)
    }

    #[test]
    fn actual_is_one_hundred_percent() {
        assert_eq!(percent(ACTUAL), 100);
        assert!(PERCENTS.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn zoom_steps_stop_at_both_ends() {
        let mut zoom = Zoom::load(None);
        for _ in 0..40 {
            zoom.step(Command::In).unwrap();
        }
        assert_eq!(percent_of(&zoom), 300);
        for _ in 0..40 {
            zoom.step(Command::Out).unwrap();
        }
        assert_eq!(percent_of(&zoom), 50);
        zoom.step(Command::Actual).unwrap();
        assert!(zoom.is_actual());
    }

    #[test]
    fn a_saved_level_comes_back_and_anything_else_is_actual_size() {
        assert_eq!(percent(parse_saved("125\n")), 125);
        for bad in ["", "99", "abc", "-1", "1.25", "70000"] {
            assert_eq!(parse_saved(bad), ACTUAL, "{bad:?}");
        }
    }

    #[test]
    fn every_zoom_in_shortcut_and_the_others_map_to_their_command() {
        let ids: Vec<_> = SHORTCUTS.iter().map(|shortcut| shortcut.id).collect();
        for (n, id) in ids.iter().enumerate() {
            assert!(!ids[..n].contains(id), "duplicate id {id}");
        }
        assert_eq!(command_for("zoom-in"), Some(Command::In));
        assert_eq!(command_for("zoom-in-equals"), Some(Command::In));
        assert_eq!(command_for("zoom-out"), Some(Command::Out));
        assert_eq!(command_for("zoom-actual"), Some(Command::Actual));
        assert_eq!(command_for("quit"), None);
    }

    #[test]
    fn a_level_is_saved_and_read_back_even_in_a_new_folder() {
        let dir = temp("roundtrip");
        let path = dir.join("nested").join(SAVED);
        let mut zoom = Zoom::load(Some(path.clone()));
        assert!(zoom.is_actual());
        zoom.step(Command::In).unwrap();
        zoom.step(Command::In).unwrap();
        assert_eq!(percent_of(&Zoom::load(Some(path))), 125);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_failed_save_keeps_the_level_in_memory() {
        // The parent of the saved path is a file, so neither creating it nor writing under it works.
        let blocker = temp("blocker");
        fs::write(&blocker, "not a folder").unwrap();
        let mut zoom = Zoom::load(Some(blocker.join(SAVED)));
        assert!(zoom.step(Command::In).is_err());
        assert_eq!(percent_of(&zoom), 110);
        assert!(zoom.step(Command::In).is_err());
        assert_eq!(percent_of(&zoom), 125);
        assert!((zoom.factor() - 1.25).abs() < 1e-9);
        fs::remove_file(blocker).unwrap();
    }

    #[test]
    fn an_unreadable_saved_file_starts_at_actual_size() {
        let dir = temp("unreadable");
        fs::create_dir_all(&dir).unwrap();
        // A folder where the file should be cannot be read as text.
        assert!(Zoom::load(Some(dir.clone())).is_actual());
        fs::remove_dir(dir).unwrap();
    }
}
