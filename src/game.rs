use rand::Rng;

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
#[derive(Default, Clone, Copy, Debug)]
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

    /// Finds any free place preferrably close to `point`.
    // ~ panics if `point` is out of the board's bounds
    pub fn find_free_any(&self, point: Cursor) -> Option<Cursor> {
        if self.num_remaining == MAX_STONES {
            return None;
        }

        macro_rules! if_free_return_cursor {
            ($x:expr, $y:expr) => {
                if self.board[$y as usize * COLS + $x as usize] == Stone::MAX {
                    return Some(Cursor {
                        x: $x as u8,
                        y: $y as u8,
                    });
                }
            };
        }

        // ~ is `point` itself free? we never get called on a free point, though.
        // if_free_return_cursor!(point.x, point.y);

        // XXX consider directly above/below/left/right closer than
        //  places on the diagonals (on the same circle)

        // ~ look for free cells in a circle around `point` with an
        // increasing radius ... thereby finding the closest free
        // board cell - if any
        for r in 1..ROWS.max(COLS) {
            // ~ check upper row
            if r <= point.y as usize {
                let y = point.y as isize - r as isize;
                for x in (point.x as usize)..(point.x as usize + r + 1).min(COLS) {
                    if_free_return_cursor!(x, y);
                }
                for x in (point.x as isize - r as isize).max(0)..(point.x as isize) {
                    if_free_return_cursor!(x, y);
                }
            }
            // ~ check right column
            if point.x as usize + r < COLS {
                let x = point.x as usize + r;
                for y in (point.y as isize - r as isize + 1).max(0)..(point.y as isize) {
                    if_free_return_cursor!(x, y);
                }
                for y in (point.y as usize)..(point.y as usize + r).min(ROWS) {
                    if_free_return_cursor!(x, y);
                }
            }
            // ~ check bottom row
            if point.y as usize + r < ROWS {
                let y = point.y as usize + r;
                for x in (point.x as usize)..(point.x as usize + r + 1).min(COLS) {
                    if_free_return_cursor!(x, y);
                }
                for x in (point.x as isize - r as isize).max(0)..(point.x as isize) {
                    if_free_return_cursor!(x, y);
                }
            }
            // ~ check left column
            if r <= point.x as usize {
                let x = point.x as usize - r;
                for y in (point.y as isize - r as isize + 1).max(0)..(point.y as isize) {
                    if_free_return_cursor!(x, y);
                }
                for y in (point.y as usize)..(point.y as usize + r).min(ROWS) {
                    if_free_return_cursor!(x, y);
                }
            }
        }
        None
    }

    /// Finds a free place next to `point` preferrably in given
    /// direction wrapping around if necessary.
    // ~ panics if `point` is out of bounds of the game's board.
    pub fn find_free_next(&self, point: Cursor, direction: Direction) -> Option<Cursor> {
        if self.num_remaining == MAX_STONES {
            return None;
        }

        // XXX rethink the behaviour; consider simplifying such that
        // the cursor doesn't wrap around, but jumps to the next free
        // cell in the next row/column

        match direction {
            Direction::North => {
                let start_y = if point.y as usize == 0 {
                    ROWS
                } else {
                    point.y as usize
                } - 1;
                let mut i = start_y * COLS + point.x as usize;
                for _ in 0..(ROWS - 1) {
                    if self.board[i] == Stone::MAX {
                        return Some(Cursor {
                            x: (i % COLS) as u8,
                            y: (i / COLS) as u8,
                        });
                    }
                    i = if i < COLS {
                        (ROWS - 1) * COLS + point.x as usize
                    } else {
                        i - COLS
                    };
                }
                self.find_free_before_vert(point.x as usize, 0)
                    .or_else(|| self.find_free_after_vert(point.x as usize, ROWS - 1))
            }
            Direction::South => {
                let mut i = (point.y as usize + 1) * COLS + point.x as usize;
                for y in point.y as usize + 1..ROWS {
                    if self.board[i] == Stone::MAX {
                        return Some(Cursor {
                            x: point.x,
                            y: y as u8,
                        });
                    }
                    i += COLS;
                }
                i = point.x as usize;
                for y in 0..point.y as usize {
                    if self.board[i] == Stone::MAX {
                        return Some(Cursor {
                            x: point.x,
                            y: y as u8,
                        });
                    }
                    i += COLS;
                }
                self.find_free_after_vert(point.x as usize, ROWS - 1)
                    .or_else(|| self.find_free_before_vert(point.x as usize, 0))
            }
            Direction::East => {
                let row = &self.board[point.y as usize * COLS..(point.y as usize + 1) * COLS];
                let (before, after) = row.split_at(point.x as usize);
                for (i, &v) in after.iter().enumerate().skip(1) {
                    if v == Stone::MAX {
                        return Some(Cursor {
                            x: point.x + i as u8,
                            y: point.y,
                        });
                    }
                }
                for (i, &v) in before.iter().enumerate() {
                    if v == Stone::MAX {
                        return Some(Cursor {
                            x: i as u8,
                            y: point.y,
                        });
                    }
                }
                self.find_free_after_horiz(COLS - 1, point.y as usize)
                    .or_else(|| self.find_free_before_horiz(0, point.y as usize))
            }
            Direction::West => {
                let row = &self.board[point.y as usize * COLS..(point.y as usize + 1) * COLS];
                let (before, after) = row.split_at(point.x as usize);
                for (i, &v) in before.iter().enumerate().rev() {
                    if v == Stone::MAX {
                        return Some(Cursor {
                            x: i as u8,
                            y: point.y,
                        });
                    }
                }
                for (i, &v) in after.iter().enumerate().skip(1).rev() {
                    if v == Stone::MAX {
                        return Some(Cursor {
                            x: point.x + i as u8,
                            y: point.y,
                        });
                    }
                }
                self.find_free_before_horiz(0, point.y as usize)
                    .or_else(|| self.find_free_after_horiz(COLS - 1, point.y as usize))
            }
        }
    }

    fn find_free_after_vert(&self, mut x: usize, mut y: usize) -> Option<Cursor> {
        if y == ROWS - 1 {
            if x == COLS - 1 {
                return None;
            } else {
                y = 0;
                x += 1;
            }
        } else {
            y += 1;
        }

        loop {
            let start_i = y * COLS + x;
            for i in (start_i..MAX_STONES).step_by(COLS) {
                if self.board[i] == Stone::MAX {
                    return Some(Cursor {
                        x: (i % COLS) as u8,
                        y: (i / COLS) as u8,
                    });
                }
            }

            if x == COLS - 1 {
                return None;
            } else {
                x += 1;
                y = 0;
            }
        }
    }

    fn find_free_before_vert(&self, mut x: usize, mut y: usize) -> Option<Cursor> {
        if y == 0 {
            if x == 0 {
                return None;
            } else {
                x -= 1;
                y = ROWS - 1;
            }
        } else {
            y -= 1;
        };

        loop {
            let mut i = y * COLS + x;
            loop {
                if self.board[i] == Stone::MAX {
                    return Some(Cursor {
                        x: (i % COLS) as u8,
                        y: (i / COLS) as u8,
                    });
                }
                if i >= COLS {
                    i -= COLS;
                } else {
                    break;
                }
            }
            if x == 0 {
                return None;
            } else {
                x -= 1;
                y = ROWS - 1;
            }
        }
    }

    fn find_free_after_horiz(&self, x: usize, y: usize) -> Option<Cursor> {
        let start_i = y * COLS + x + 1;
        let after = &self.board[start_i..];
        for (i, &x) in after.iter().enumerate() {
            if x == Stone::MAX {
                return Some(Cursor {
                    x: ((start_i + i) % COLS) as u8,
                    y: ((start_i + i) / COLS) as u8,
                });
            }
        }
        None
    }

    fn find_free_before_horiz(&self, x: usize, y: usize) -> Option<Cursor> {
        let end_i = y * COLS + x;
        let before = &self.board[0..end_i];
        for (i, &x) in before.iter().rev().enumerate() {
            if x == Stone::MAX {
                return Some(Cursor {
                    x: ((end_i - i - 1) % COLS) as u8,
                    y: ((end_i - i - 1) / COLS) as u8,
                });
            }
        }
        None
    }

    /// Artificially sets the board to finished state.
    #[cfg(feature = "dev")]
    pub fn set_finished(&mut self, state: Finished) {
        match state {
            Finished::Success => {
                for i in 0..self.board.len() {
                    self.board[i] = Stone::MAX;
                }
                self.num_remaining = 0;
            }
            Finished::Failure => {
                let mut i = 0;
                for r in 0..ROWS {
                    for c in 0..COLS {
                        self.board[i] = ((r + c) % COLS) as u8;
                        i += 1;
                    }
                }
                self.num_remaining = MAX_STONES;
            }
        }
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
