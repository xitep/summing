use argh::FromArgs;

/// A "summing" game.
#[derive(FromArgs)]
pub struct Options {
    /// seed to initialize the random number generator with
    #[argh(option, short = 's')]
    pub seed: Option<u64>,

    /// draw with full-width characters
    #[argh(switch, short = 'w')]
    pub wide: bool,

    /// loads a predefined board
    #[cfg(feature = "dev")]
    #[argh(option)]
    pub board: Option<std::path::PathBuf>,
}

pub fn from_env() -> Options {
    argh::from_env()
}
