use std::collections::BTreeSet;

use rand::{
    seq::{IteratorRandom, SliceRandom},
    thread_rng, SeedableRng,
};
use rand_chacha::ChaCha12Rng;

#[derive(Debug, Clone, PartialEq)]
pub struct GameOptions {
    pub size: (usize, usize),
    pub safe_pos: Option<(usize, usize)>,
    pub mines: usize,
    pub seed: Option<u64>,
}

impl Default for GameOptions {
    fn default() -> Self {
        Self::easy()
    }
}

impl GameOptions {
    pub fn easy() -> Self {
        Self {
            size: (9, 9),
            safe_pos: None,
            mines: 10,
            seed: None,
        }
    }

    pub fn medium() -> Self {
        Self {
            size: (16, 16),
            safe_pos: None,
            mines: 40,
            seed: None,
        }
    }

    pub fn hard() -> Self {
        Self {
            size: (30, 16),
            safe_pos: None,
            mines: 99,
            seed: None,
        }
    }

    /// Panics when width, height or mines is zero, or when every cell would be filled with mine
    pub fn build(self) -> GameState {
        let (w, h) = self.size;
        if w < 1 || h < 1 || self.mines < 1 || w * h <= self.mines {
            panic!(
                "width, height and mines shouldn't be zero and at least one cell should be empty"
            )
        }
        let mut rng = match self.seed {
            Some(seed) => ChaCha12Rng::seed_from_u64(seed),
            None => ChaCha12Rng::from_rng(thread_rng()).unwrap(),
        };
        let mut mines = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .choose_multiple(&mut rng, self.mines + 1);
        if let Some(safe_pos) = self.safe_pos {
            if let Some(p) = mines.iter().position(|&p| p == safe_pos) {
                mines.remove(p);
            }
        }
        if mines.len() > self.mines {
            mines.shuffle(&mut rng);
            mines.pop();
        }
        use CellState::Unopened;
        let mut state = GameState {
            mines: (0..h).map(|_| (0..w).map(|_| false).collect()).collect(),
            cells: (0..h).map(|_| (0..w).map(|_| Unopened).collect()).collect(),
        };
        for (x, y) in mines {
            state.mines[y][x] = true;
        }
        state
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub enum CellState {
    Unopened,
    Flagged,
    Questioned,
    Opened,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct GameState {
    pub mines: Vec<Vec<bool>>,
    pub cells: Vec<Vec<CellState>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub enum GameResult {
    Win,
    Lose,
    Playing,
}

impl GameState {
    pub fn width(&self) -> usize {
        self.mines[0].len()
    }

    pub fn height(&self) -> usize {
        self.mines.len()
    }

    pub fn is_mine(&self, x: usize, y: usize) -> bool {
        self.mines[y][x]
    }

    pub fn mines(&self) -> usize {
        let mut mines = 0;
        for y in 0..self.height() {
            for x in 0..self.width() {
                if self.is_mine(x, y) {
                    mines += 1;
                }
            }
        }
        mines
    }

    pub fn flags(&self) -> usize {
        let mut flags = 0;
        for y in 0..self.height() {
            for x in 0..self.width() {
                if self.is_flag(x, y) {
                    flags += 1;
                }
            }
        }
        flags
    }

    pub fn cell_state(&self, x: usize, y: usize) -> CellState {
        self.cells[y][x]
    }

    pub fn set_cell_state(&mut self, x: usize, y: usize, state: CellState) {
        self.cells[y][x] = state;
    }

    pub fn nearby_mines(&self, x: usize, y: usize) -> u8 {
        let mut nearby_mines = 0;
        let x = x as i32;
        let y = y as i32;
        for y1 in [y - 1, y, y + 1] {
            if y1 < 0 || y1 >= self.height() as i32 {
                continue;
            }
            for x1 in [x - 1, x, x + 1] {
                if x1 < 0 || x1 >= self.width() as i32 {
                    continue;
                }
                if (!(x1 == x && y1 == y)) && self.is_mine(x1 as usize, y1 as usize) {
                    nearby_mines += 1;
                }
            }
        }
        nearby_mines
    }

    pub fn is_flag(&self, x: usize, y: usize) -> bool {
        self.cells[y][x] == CellState::Flagged
    }

    pub fn nearby_flags(&self, x: usize, y: usize) -> u8 {
        let mut nearby_flags = 0;
        let x = x as i32;
        let y = y as i32;
        for y1 in [y - 1, y, y + 1] {
            if y1 < 0 || y1 >= self.height() as i32 {
                continue;
            }
            for x1 in [x - 1, x, x + 1] {
                if x1 < 0 || x1 >= self.width() as i32 {
                    continue;
                }
                if (!(x1 == x && y1 == y)) && self.is_flag(x1 as usize, y1 as usize) {
                    nearby_flags += 1;
                }
            }
        }
        nearby_flags
    }

    pub fn is_opened(&self, x: usize, y: usize) -> bool {
        self.cells[y][x] == CellState::Opened
    }

    pub fn is_exploded(&self, x: usize, y: usize) -> bool {
        self.is_opened(x, y) && self.is_mine(x, y)
    }

    pub fn game_result(&self) -> GameResult {
        let mut cont = false;
        for y in 0..self.height() {
            for x in 0..self.width() {
                match (self.is_opened(x, y), self.is_mine(x, y)) {
                    (false, false) => cont = true,
                    (true, false) => (),
                    (false, true) => (),
                    (true, true) => return GameResult::Lose,
                }
            }
        }
        if cont {
            GameResult::Playing
        } else {
            GameResult::Win
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CellView {
    Unopened,
    Hovered,
    Pushed,
    Flagged,
    Questioned,
    Opened(u8),
    Mine,
    WrongMine,
    Exploded,
}

#[derive(Debug, Clone, Copy)]
pub enum Gesture {
    Hover(usize, usize),
    LeftOrRightPush(usize, usize),
    MidPush(usize, usize),
    None,
}

#[derive(Debug, Clone)]
pub struct GameView {
    pub state: GameState,
    pub cells: Vec<Vec<CellView>>,
    pub result: GameResult,
    pub gesture: Gesture,
    pub mines: usize,
    pub flags: usize,
}

impl From<GameState> for GameView {
    fn from(state: GameState) -> Self {
        let result = GameResult::Playing;
        let cells = (0..state.height())
            .map(|_| (0..state.width()).map(|_| CellView::Unopened).collect())
            .collect();
        let mines = state.mines();
        let mut this = Self {
            state,
            cells,
            result,
            gesture: Gesture::None,
            mines,
            flags: 0,
        };
        this.refresh_game_result();
        this.refresh_all_cell();
        this
    }
}

impl GameView {
    pub fn width(&self) -> usize {
        self.state.width()
    }

    pub fn height(&self) -> usize {
        self.state.width()
    }

    fn refresh_game_result(&mut self) {
        self.result = self.state.game_result();
    }

    fn refresh_all_cell(&mut self) {
        for y in 0..self.state.height() {
            for x in 0..self.state.width() {
                self.refresh_cell(x, y);
            }
        }
    }

    fn refresh_3x3_cell(&mut self, x: usize, y: usize) {
        let x = x as i32;
        let y = y as i32;
        for y1 in [y - 1, y, y + 1] {
            if y1 < 0 || y1 >= self.height() as i32 {
                continue;
            }
            for x1 in [x - 1, x, x + 1] {
                if x1 < 0 || x1 >= self.width() as i32 {
                    continue;
                }
                self.refresh_cell(x1 as usize, y1 as usize);
            }
        }
    }

    fn refresh_gesture(&mut self, gesture: Gesture) {
        match gesture {
            Gesture::Hover(x, y) | Gesture::LeftOrRightPush(x, y) | Gesture::MidPush(x, y) => {
                self.refresh_3x3_cell(x, y);
            }
            Gesture::None => {}
        }
    }

    fn refresh_cell(&mut self, x: usize, y: usize) {
        use CellView::*;
        use GameResult::*;
        let cell_view = match (
            self.result,
            self.state.is_mine(x, y),
            self.state.cell_state(x, y),
        ) {
            (Win, true, CellState::Unopened) => Flagged,
            (Win, true, CellState::Flagged) => Flagged,
            (Win, true, CellState::Questioned) => Flagged,
            (Win, true, CellState::Opened) => unreachable!(),
            (Win, false, CellState::Opened) => Opened(self.state.nearby_mines(x, y)),
            (Win, false, _) => unreachable!(),
            (Lose, true, CellState::Unopened) => Mine,
            (Lose, true, CellState::Flagged) => Flagged,
            (Lose, true, CellState::Questioned) => Questioned,
            (Lose, true, CellState::Opened) => Exploded,
            (Lose, false, CellState::Unopened) => Unopened,
            (Lose, false, CellState::Flagged) => WrongMine,
            (Lose, false, CellState::Questioned) => Questioned,
            (Lose, false, CellState::Opened) => Opened(self.state.nearby_mines(x, y)),
            (Playing, true, CellState::Unopened) => Unopened,
            (Playing, true, CellState::Flagged) => Flagged,
            (Playing, true, CellState::Questioned) => Questioned,
            (Playing, true, CellState::Opened) => unreachable!(),
            (Playing, false, CellState::Unopened) => Unopened,
            (Playing, false, CellState::Flagged) => Flagged,
            (Playing, false, CellState::Questioned) => Questioned,
            (Playing, false, CellState::Opened) => Opened(self.state.nearby_mines(x, y)),
        };
        let cell_view = if self.result == Playing && cell_view == Unopened {
            match self.gesture {
                Gesture::Hover(x0, y0) if x == x0 && y == y0 => Hovered,
                Gesture::LeftOrRightPush(x0, y0) if x == x0 && y == y0 => Pushed,
                Gesture::MidPush(x0, y0) if x == x0 && y == y0 => Hovered,
                Gesture::MidPush(x0, y0)
                    if x as i32 - 1 <= x0 as i32
                        && x0 <= x + 1
                        && y as i32 - 1 <= y0 as i32
                        && y0 <= y + 1 =>
                {
                    Pushed
                }
                _ => Unopened,
            }
        } else {
            cell_view
        };
        self.cells[y][x] = cell_view;
    }

    pub fn left_click(&mut self, x: usize, y: usize) {
        if self.result != GameResult::Playing {
            return;
        }
        use CellState::*;
        if self.state.cell_state(x, y) != Unopened {
            return;
        }
        if self.state.is_mine(x, y) {
            self.state.set_cell_state(x, y, Opened);
        } else {
            let mut cells_to_left_click = BTreeSet::new();
            cells_to_left_click.insert((x, y));
            while let Some(cell) = cells_to_left_click.pop_first() {
                let (x, y) = cell;
                if self.state.cell_state(x, y) == Unopened {
                    self.state.set_cell_state(x, y, Opened);
                    self.refresh_cell(x, y);
                    if self.state.nearby_mines(x, y) == 0 {
                        let x = x as i32;
                        let y = y as i32;
                        for y1 in [y - 1, y, y + 1] {
                            if y1 < 0 || y1 >= self.height() as i32 {
                                continue;
                            }
                            for x1 in [x - 1, x, x + 1] {
                                if x1 < 0 || x1 >= self.width() as i32 {
                                    continue;
                                }
                                if !(x1 == x && y1 == y) {
                                    cells_to_left_click.insert((x1 as usize, y1 as usize));
                                }
                            }
                        }
                    }
                }
            }
        }
        self.refresh_game_result();
        if self.result != GameResult::Playing {
            self.refresh_all_cell();
        }
    }

    pub fn right_click(&mut self, x: usize, y: usize) {
        if self.result != GameResult::Playing {
            return;
        }
        use CellState::*;
        let cell_state = self.state.cell_state(x, y);
        let new_cell_state = match cell_state {
            Unopened => {
                self.flags += 1;
                Flagged
            }
            Flagged => {
                self.flags -= 1;
                Questioned
            }
            Questioned => Unopened,
            Opened => return,
        };
        self.state.set_cell_state(x, y, new_cell_state);
        self.refresh_cell(x, y);
    }

    pub fn middle_click(&mut self, x: usize, y: usize) {
        if self.result != GameResult::Playing {
            return;
        }
        use CellState::*;
        if self.state.cell_state(x, y) != Opened
            || self.state.nearby_mines(x, y) != self.state.nearby_flags(x, y)
        {
            return;
        }
        let x = x as i32;
        let y = y as i32;
        for y1 in [y - 1, y, y + 1] {
            if y1 < 0 || y1 >= self.height() as i32 {
                continue;
            }
            for x1 in [x - 1, x, x + 1] {
                if x1 < 0 || x1 >= self.width() as i32 {
                    continue;
                }
                if (!(x1 == x && y1 == y))
                    && self.state.cell_state(x1 as usize, y1 as usize) == Unopened
                {
                    self.state.set_cell_state(x1 as usize, y1 as usize, Opened);
                }
            }
        }
        self.refresh_game_result();
        if self.result != GameResult::Playing {
            self.refresh_all_cell();
        } else {
            self.refresh_3x3_cell(x as usize, y as usize);
        }
    }
    pub fn gesture(&mut self, gesture: Gesture) {
        let previous_gesture = self.gesture;
        self.gesture = gesture;
        self.refresh_gesture(previous_gesture);
        self.refresh_gesture(gesture);
    }

    pub fn is_draggable(&self, x: usize, y: usize) -> bool {
        match self.result {
            GameResult::Win | GameResult::Lose => true,
            GameResult::Playing => matches!(self.cells[y][x], CellView::Opened(_)),
        }
    }

    pub fn iter(&self) -> GameViewIter {
        GameViewIter {
            game_view: self,
            width: self.state.width(),
            height: self.state.height(),
            x: 0,
            y: 0,
        }
    }
}

#[derive(Debug)]
pub struct GameViewIter<'a> {
    game_view: &'a GameView,
    width: usize,
    height: usize,
    x: usize,
    y: usize,
}

impl Iterator for GameViewIter<'_> {
    type Item = (usize, usize, CellView);

    fn next(&mut self) -> Option<Self::Item> {
        if self.x >= self.width {
            self.x = 0;
            self.y += 1;
        }
        if self.y >= self.height {
            return None;
        }
        let result = (self.x, self.y, self.game_view.cells[self.y][self.x]);
        self.x += 1;
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_game() {
        let options = GameOptions {
            size: (3, 3),
            safe_pos: None,
            mines: 3,
            seed: Some(1),
        };
        let state = options.build();
        assert_eq!(
            state,
            GameState {
                mines: vec![
                    vec![true, false, false],
                    vec![false, false, false],
                    vec![true, true, false]
                ],
                cells: vec![
                    vec![
                        CellState::Unopened,
                        CellState::Unopened,
                        CellState::Unopened
                    ];
                    3
                ],
            }
        )
    }

    #[test]
    fn game_view() {
        let mut view = GameView::from(
            GameOptions {
                size: (3, 3),
                safe_pos: None,
                mines: 3,
                seed: Some(1),
            }
            .build(),
        );
        view.left_click(1, 1);
        assert_eq!(
            view.state.cells,
            vec![
                vec![
                    CellState::Unopened,
                    CellState::Unopened,
                    CellState::Unopened,
                ],
                vec![CellState::Unopened, CellState::Opened, CellState::Unopened,],
                vec![
                    CellState::Unopened,
                    CellState::Unopened,
                    CellState::Unopened,
                ]
            ],
        );
        assert_eq!(
            view.cells,
            vec![
                vec![CellView::Unopened, CellView::Unopened, CellView::Unopened],
                vec![CellView::Unopened, CellView::Opened(3), CellView::Unopened],
                vec![CellView::Unopened, CellView::Unopened, CellView::Unopened]
            ]
        );
        assert_eq!(view.result, GameResult::Playing);
        view.right_click(2, 1);
        assert_eq!(
            view.state.cells,
            vec![
                vec![
                    CellState::Unopened,
                    CellState::Unopened,
                    CellState::Unopened,
                ],
                vec![CellState::Unopened, CellState::Opened, CellState::Flagged,],
                vec![
                    CellState::Unopened,
                    CellState::Unopened,
                    CellState::Unopened,
                ]
            ],
        );
        assert_eq!(
            view.cells,
            vec![
                vec![CellView::Unopened, CellView::Unopened, CellView::Unopened],
                vec![CellView::Unopened, CellView::Opened(3), CellView::Flagged],
                vec![CellView::Unopened, CellView::Unopened, CellView::Unopened]
            ]
        );
        assert_eq!(view.result, GameResult::Playing);
        view.left_click(0, 0);
        assert_eq!(
            view.state.cells,
            vec![
                vec![CellState::Opened, CellState::Unopened, CellState::Unopened,],
                vec![CellState::Unopened, CellState::Opened, CellState::Flagged,],
                vec![
                    CellState::Unopened,
                    CellState::Unopened,
                    CellState::Unopened,
                ]
            ],
        );
        assert_eq!(
            view.cells,
            vec![
                vec![CellView::Exploded, CellView::Unopened, CellView::Unopened],
                vec![CellView::Unopened, CellView::Opened(3), CellView::WrongMine],
                vec![CellView::Mine, CellView::Mine, CellView::Unopened]
            ]
        );
        assert_eq!(view.result, GameResult::Lose);
    }
}
