use argh::FromArgs;

/// A "summing" game.
#[derive(FromArgs)]
pub struct Options {
    /// seed to initialize the random number generator with
    #[argh(option)]
    pub seed: Option<u64>,

    /// start start of the board; useful for development purposes
    #[cfg(feature = "dev")]
    #[argh(option, from_str_fn(parse_start_state))]
    pub start_state: Option<StartState>,
}

#[cfg(feature = "dev")]
pub enum StartState {
    Success,
    Failure,
}

#[cfg(feature = "dev")]
fn parse_start_state(s: &str) -> Result<StartState, String> {
    match s {
        "success" => Ok(StartState::Success),
        "failure" => Ok(StartState::Failure),
        _ => Err("Not a start state; try \"success\" or \"failure\"".into()),
    }
}

pub fn from_env() -> Options {
    argh::from_env()
}
