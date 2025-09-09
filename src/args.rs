use argh::FromArgs;
use rand::TryRngCore;

/// A "summing" game.
#[derive(FromArgs)]
pub struct Options {
    /// seed to initialize the random number generator with
    #[argh(option, short = 's', default = "default_seed()")]
    pub seed: u64,

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

fn default_seed() -> u64 {
    rand::rngs::OsRng
        .try_next_u64()
        .expect("os rng not ready (yet)")
}
