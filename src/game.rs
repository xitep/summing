use rand::{seq::SliceRandom, Rng};

// ~ 0-9 or u8::MAX
type Stone = u8;

const NUM_STONES: u8 = 10;
const ROWS: usize = 9;
const COLS: usize = 9;

const MAX_STONES: usize = ROWS * COLS;

/// Game board state
pub struct Game<R> {
    // ~ random number generator
    rng: R,
    // ~ four stones to be served next (left to right)
    nexts: u32,
    // ~ number of stones still on the board; zero when the game is
    // finished; `MAX_STONES` if the board is full and no new
    // placement is possible
    num_remaining: usize,
    // ~ number of (user) placed stones, ie. the "score"
    num_placed: usize,
    // ~ the board of stones; rows of columns
    board: [Stone; MAX_STONES],
}

pub enum Finished {
    /// The game has been finished successfully
    Success,
    /// The game finished with all cells being occupied, but none
    /// cancelling out each other, such that placement of further
    /// stones would be possible.  This is considered a defeat /
    /// failure.
    Failure,
}

/// Cursor into the game's board
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cursor {
    pub x: u8,
    pub y: u8,
}

pub enum Direction {
    North,
    South,
    East,
    West,
}

// ~ may panic if `stone >= NUM_STONES`
fn stone_label(stone: u8) -> &'static str {
    &"0123456789"[stone as usize..(stone as usize + 1)]
}

impl<R> Game<R> {
    pub fn rows(&self) -> usize {
        ROWS
    }

    pub fn cols(&self) -> usize {
        COLS
    }

    pub fn nexts(&self) -> impl Iterator<Item = &'static str> {
        [
            ((self.nexts & 0x_ff00_0000) >> 24) as u8,
            ((self.nexts & 0x_00ff_0000) >> 16) as u8,
            ((self.nexts & 0x_0000_ff00) >> 8) as u8,
            (self.nexts & 0x_0000_00ff) as u8,
        ]
        .map(stone_label)
        .into_iter()
    }

    /// Tells the number of placed stones so far.
    pub fn num_placed(&self) -> usize {
        self.num_placed
    }

    // ~ panics if `row` or `col` are out of bounds.
    pub fn get(&self, row: usize, col: usize) -> Option<&'static str> {
        let v = self.board[row * COLS + col];
        if v == Stone::MAX {
            None
        } else {
            Some(stone_label(v))
        }
    }

    /// Finds a free place next to `point` preferrably in given
    /// direction.
    // ~ panics if `point` is out of bounds of the game's board.
    pub fn find_free_next(&self, point: Cursor, direction: Direction) -> Option<Cursor> {
        if self.num_remaining == MAX_STONES {
            return None;
        }

        macro_rules! if_free_return_cursor {
            ($index:expr, $value:expr) => {
                if $value == Stone::MAX {
                    return Some(Cursor {
                        x: ($index % COLS) as u8,
                        y: ($index / COLS) as u8,
                    });
                }
            };
        }

        match direction {
            Direction::North => {
                let (mut x, mut y) = if point.y as usize == 0 {
                    if point.x as usize == 0 {
                        (COLS - 1, ROWS - 1)
                    } else {
                        (point.x as usize - 1, ROWS - 1)
                    }
                } else {
                    (point.x as usize, point.y as usize - 1)
                };
                for _ in 0..=COLS {
                    for y in (0..=y).rev() {
                        let i = y * COLS + x;
                        if_free_return_cursor!(i, self.board[i]);
                    }
                    y = ROWS - 1;
                    if x == 0 {
                        x = COLS - 1;
                    } else {
                        x -= 1;
                    }
                }
            }
            Direction::South => {
                let (mut x, mut y) = if point.y as usize == ROWS - 1 {
                    if point.x as usize == COLS - 1 {
                        (0, 0)
                    } else {
                        (point.x as usize + 1, 0)
                    }
                } else {
                    (point.x as usize, point.y as usize + 1)
                };
                for _ in 0..=COLS {
                    let mut i = y * COLS + x;
                    for _ in y..ROWS {
                        if_free_return_cursor!(i, self.board[i]);
                        i += COLS;
                    }
                    y = 0;
                    x = (x + 1) % COLS;
                }
            }
            Direction::East => {
                let point_i = point.y as usize * COLS + point.x as usize;
                let (before, after) = self.board.split_at(point_i);
                for (i, &v) in after.iter().enumerate().skip(1) {
                    if_free_return_cursor!(point_i + i, v);
                }
                for (i, &v) in before.iter().enumerate() {
                    if_free_return_cursor!(i, v);
                }
            }
            Direction::West => {
                let point_i = point.y as usize * COLS + point.x as usize;
                let (before, after) = self.board.split_at(point_i);
                for (i, &v) in before.iter().enumerate().rev() {
                    if_free_return_cursor!(i, v);
                }
                for (i, &v) in after.iter().enumerate().skip(1).rev() {
                    if_free_return_cursor!(point_i + i, v);
                }
            }
        }
        if self.board[point.y as usize * COLS + point.x as usize] == Stone::MAX {
            Some(point)
        } else {
            None
        }
    }

    /// Loads the board from a textual presentation. Example:
    ///
    /// ```
    /// .........
    /// .1234678.
    /// ...7.0.2.
    /// .1234678.
    /// .123.679.
    /// .1...638.
    /// .12.4670.
    /// .1234678.
    /// .........
    /// ```
    ///
    /// No guarantee about the state of the game is provided if the
    /// given content cannot be parsed correctly.
    #[cfg(feature = "dev")]
    pub fn load_from_reader<S: std::io::BufRead>(&mut self, rdr: S) -> anyhow::Result<()> {
        for (y, line) in rdr.lines().enumerate().take(9) {
            for (x, c) in line?.bytes().enumerate().take(9) {
                self.board[y * COLS + x] = if c.is_ascii_digit() {
                    c - b'0'
                } else if c == b' ' || c == b'.' {
                    Stone::MAX
                } else {
                    anyhow::bail!("invalid character '{c}' [line: {y} / column: {x}");
                };
            }
        }
        self.num_remaining = self.board.iter().filter(|&&c| c != Stone::MAX).count();
        Ok(())
    }

    /// Determines whether the game is considered over.
    pub fn is_finished(&self) -> Option<Finished> {
        match self.num_remaining {
            0 => Some(Finished::Success),
            MAX_STONES => Some(Finished::Failure),
            _ => None,
        }
    }
}

impl<R: Rng> Game<R> {
    pub fn new(mut rng: R) -> Self {
        let nexts = (rng.random_range::<u32, _>(0..NUM_STONES as u32) << 24)
            | (rng.random_range::<u32, _>(0..NUM_STONES as u32) << 16)
            | (rng.random_range::<u32, _>(0..NUM_STONES as u32) << 8)
            | rng.random_range::<u32, _>(0..NUM_STONES as u32);
        Self {
            board: new_board(&mut rng),
            num_remaining: (ROWS - 2) * (COLS - 2),
            num_placed: 0,
            nexts,
            rng,
        }
    }

    /// Reinitializes this game for a new round from scratch.
    pub fn reinit(&mut self) {
        self.board = new_board(&mut self.rng);
        self.num_placed = 0;
        self.num_remaining = (ROWS - 2) * (COLS - 2);
    }

    /// Finds any free place preferrably close to `point`.
    // ~ panics if `point` is out of the board's bounds
    pub fn find_free_any(&mut self, point: Cursor) -> Option<Cursor> {
        if self.num_remaining == MAX_STONES {
            return None;
        }
        macro_rules! if_free_return_cursor {
            ($x:expr, $y:expr, $label:literal) => {
                if self.board[$y as usize * COLS + $x as usize] == Stone::MAX {
                    return Some(Cursor {
                        x: $x as u8,
                        y: $y as u8,
                    });
                }
            };
        }
        // ~ is `point` itself free?
        if_free_return_cursor!(point.x, point.y, "point");
        // ~ start out in random order to get some variability
        let directions = {
            let mut ds = [
                Direction::North,
                Direction::East,
                Direction::South,
                Direction::West,
            ];
            ds.shuffle(&mut self.rng);
            ds
        };
        // ~ look for free cells in a circle around `point` with an
        // increasing radius. the number of needed jumps (ups/downs,
        // lefts/rights) from `point` to a target cell is the distance
        // which we strive to be minimal in the finally suggested cell
        let (x, y) = (point.x as usize, point.y as usize);
        for r in 1..ROWS.max(COLS) {
            for o in 0..=r {
                for d in &directions {
                    match d {
                        Direction::North => {
                            if y >= r {
                                if x + o < COLS {
                                    if_free_return_cursor!(x + o, y - r, "north right");
                                }
                                if o > 0 && x >= o {
                                    if_free_return_cursor!(x - o, y - r, "north left");
                                }
                            }
                        }
                        Direction::East => {
                            // ~ corners are checked by "north" and "south"
                            if o != r && x + r < COLS {
                                if y + o < ROWS {
                                    if_free_return_cursor!(x + r, y + o, "east  down");
                                }
                                if o > 0 && y >= o {
                                    if_free_return_cursor!(x + r, y - o, "east  up");
                                }
                            }
                        }
                        Direction::South => {
                            if y + r < ROWS {
                                if x >= o {
                                    if_free_return_cursor!(x - o, y + r, "south left");
                                }
                                if o > 0 && x + o < COLS {
                                    if_free_return_cursor!(x + o, y + r, "south right");
                                }
                            }
                        }
                        Direction::West => {
                            // ~ corners are checked by "north" and "south" already
                            if o != r && x >= r {
                                if y >= o {
                                    if_free_return_cursor!(x - r, y - o, "west  up");
                                }
                                if o > 0 && y + o < ROWS {
                                    if_free_return_cursor!(x - r, y + o, "west  down");
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Assuming the cell at `point` is free, attempt to place the
    /// next stone (from `nexts`) to it, returning `true` if the stone
    /// was placed and now occupies the cell, or `false` if it cleared
    /// all neighbours and the cell at `point` was left free.
    // ~ panics if `point` is out of bounds
    pub fn place_next(&mut self, point: Cursor) -> bool {
        let (idxs, cnt, sum) = {
            // ~ row above `point`
            let (x, y) = (point.x as usize, point.y as usize);
            let mut idxs = [usize::MAX; 8];
            let i = y * COLS + x;
            if y > 0 {
                if x > 0 {
                    idxs[0] = i - COLS - 1;
                }
                idxs[1] = i - COLS;
                if x < (COLS - 1) {
                    idxs[2] = i - COLS + 1;
                }
            }
            // ~ row of `point`
            if x > 0 {
                idxs[3] = i - 1;
            }
            if x < (COLS - 1) {
                idxs[4] = i + 1;
            }
            // ~ row below `point`
            if y < (ROWS - 1) {
                if x > 0 {
                    idxs[5] = i + COLS - 1;
                }
                idxs[6] = i + COLS;
                if x < (COLS - 1) {
                    idxs[7] = i + COLS + 1;
                }
            }

            let (cnt, sum) = idxs
                .iter()
                .filter(|&&i| i != usize::MAX && self.board[i] != Stone::MAX)
                .map(|&i| self.board[i] as usize)
                .fold((0, 0), |(cnt, sum), v| (cnt + 1, sum + v));
            (idxs, cnt, (sum % 10))
        };

        let n = ((self.nexts & 0x_ff00_0000) >> 24) as u8;
        self.nexts = (self.nexts << 8) | self.rng.random_range::<u32, _>(0..NUM_STONES as u32);

        let cleared = if cnt > 0 && n as usize == sum {
            idxs.into_iter().filter(|&i| i != usize::MAX).for_each(|i| {
                self.board[i] = Stone::MAX;
            });
            self.num_remaining -= cnt;
            false
        } else {
            self.board[point.y as usize * COLS + point.x as usize] = n;
            self.num_remaining += 1;
            true
        };
        self.num_placed = self.num_placed.saturating_add(1);
        cleared
    }
}

fn new_board<R: Rng>(rng: &mut R) -> [Stone; MAX_STONES] {
    let mut xs = [0u8; ROWS * COLS];
    // ~ first row
    (0..COLS).for_each(|i| {
        xs[i] = Stone::MAX;
    });
    // ~ middle cells
    for row in 1..(ROWS - 1) {
        xs[row * COLS] = Stone::MAX;
        for col in 1..COLS - 1 {
            xs[row * COLS + col] = rng.random_range::<u8, _>(0..NUM_STONES);
        }
        xs[row * COLS + COLS - 1] = Stone::MAX;
    }
    // ~ last row
    (((ROWS - 1) * COLS)..(((ROWS - 1) * COLS) + COLS)).for_each(|i| {
        xs[i] = Stone::MAX;
    });
    xs
}

#[cfg(test)]
mod tests {
    use super::{Cursor, Game, Stone, COLS, ROWS};

    struct ConstantRng;

    impl rand::RngCore for ConstantRng {
        fn next_u32(&mut self) -> u32 {
            0
        }

        fn next_u64(&mut self) -> u64 {
            0
        }

        fn fill_bytes(&mut self, dst: &mut [u8]) {
            for b in dst {
                *b = 0;
            }
        }
    }

    fn make_board(board: [&str; ROWS]) -> Game<ConstantRng> {
        let mut game = Game::new(ConstantRng);
        for (y, line) in board.iter().enumerate() {
            for (x, c) in line.chars().enumerate() {
                game.board[y * COLS + x] = if c.is_ascii_digit() {
                    (c as usize - '0' as usize) as u8
                } else {
                    Stone::MAX
                };
            }
        }
        game
    }

    #[test]
    fn test_find_free_any_full_board() {
        let mut game = make_board([
            "000000000",
            "111111111",
            "222222222",
            "333333333",
            "444444444",
            "555555555",
            "666666666",
            "777777777",
            "888888888",
        ]);
        assert_eq!(None, game.find_free_any(Cursor { x: 0, y: 0 }));
    }

    #[test]
    fn test_find_free_any_single_free_cell() {
        for y in 0..ROWS {
            for x in 0..COLS {
                let mut game = make_board([
                    "000000000",
                    "111111111",
                    "222222222",
                    "333333333",
                    "444444444",
                    "555555555",
                    "666666666",
                    "777777777",
                    "888888888",
                ]);
                game.board[y * COLS + x] = Stone::MAX;
                // ~ start at the free cell itself
                assert_eq!(
                    Some(Cursor {
                        x: x as u8,
                        y: y as u8
                    }),
                    game.find_free_any(Cursor {
                        x: x as u8,
                        y: y as u8
                    })
                );
                // ~ start somewhere on a non-free cell
                assert_eq!(
                    Some(Cursor {
                        x: x as u8,
                        y: y as u8
                    }),
                    game.find_free_any(Cursor {
                        x: ((x + 1) % COLS) as u8,
                        y: y as u8
                    })
                );
            }
        }
    }

    #[test]
    fn test_find_free_any_closest_0() {
        let mut game = make_board([
            "000000.00",
            "111111111",
            "222222222",
            "333333333",
            "444444444",
            "555555555",
            "666666666",
            "777777777",
            "88.888888",
        ]);
        // ~ starting on a free cell, should deliver that cell
        assert_eq!(
            Some(Cursor { x: 6, y: 0 }),
            game.find_free_any(Cursor { x: 6, y: 0 })
        );
        assert_eq!(
            Some(Cursor { x: 2, y: 8 }),
            game.find_free_any(Cursor { x: 2, y: 8 })
        );
        // ~ now, let's probe some other start points (that should
        // yield the lower left free cell as its closest)
        for p in [(0, 8), (8, 8), (3, 4), (6, 6)] {
            assert_eq!(
                Some(Cursor { x: 2, y: 8 }),
                game.find_free_any(Cursor { x: p.0, y: p.1 })
            );
        }
    }
}
