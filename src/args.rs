use argh::FromArgs;

/// A "summing" game.
#[derive(FromArgs)]
pub struct Options {
    /// seed to initialize the random number generator with
    #[argh(option)]
    pub seed: Option<u64>,

    /// loads a predefined board
    #[cfg(feature = "dev")]
    #[argh(option)]
    pub board: Option<std::path::PathBuf>,
}

pub fn from_env() -> Options {
    argh::from_env()
}
