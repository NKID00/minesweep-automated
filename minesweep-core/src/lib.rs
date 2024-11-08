mod solve;

use std::{
    collections::{BTreeSet, HashSet},
    ops::{Deref, DerefMut},
};

use rand::{
    seq::{IteratorRandom, SliceRandom},
    thread_rng, RngCore, SeedableRng,
};
use rand_chacha::ChaCha12Rng;
use solve::SolveResult;

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
    Custom {
        width: usize,
        height: usize,
        mines: usize,
    },
}

impl Difficulty {
    pub fn width(&self) -> usize {
        use Difficulty::*;
        match self {
            Easy => 9,
            Medium => 16,
            Hard => 30,
            Custom {
                width,
                height: _,
                mines: _,
            } => *width,
        }
    }

    pub fn height(&self) -> usize {
        use Difficulty::*;
        match self {
            Easy => 9,
            Medium => 16,
            Hard => 16,
            Custom {
                width: _,
                height,
                mines: _,
            } => *height,
        }
    }

    pub fn mines(&self) -> usize {
        use Difficulty::*;
        match self {
            Easy => 10,
            Medium => 40,
            Hard => 99,
            Custom {
                width: _,
                height: _,
                mines,
            } => *mines,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct GameOptions {
    pub difficulty: Difficulty,
    pub safe_pos: Option<(usize, usize)>,
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
            difficulty: Difficulty::Easy,
            safe_pos: None,
            seed: None,
        }
    }

    pub fn medium() -> Self {
        Self {
            difficulty: Difficulty::Medium,
            safe_pos: None,
            seed: None,
        }
    }

    pub fn hard() -> Self {
        Self {
            difficulty: Difficulty::Hard,
            safe_pos: None,
            seed: None,
        }
    }

    /// Panics when width, height or mines is zero, or when every cell would be filled with mine
    pub fn build(mut self) -> GameState {
        let w = self.difficulty.width();
        let h = self.difficulty.height();
        let mines = self.difficulty.mines();
        if w < 1 || h < 1 || mines < 1 || w * h <= mines {
            panic!(
                "width, height and mines shouldn't be zero and at least one cell should be empty"
            )
        }
        let seed = match self.seed {
            Some(seed) => seed,
            None => thread_rng().next_u64(),
        };
        self.seed = Some(seed);
        let mut rng = ChaCha12Rng::seed_from_u64(seed);
        let mut mines_pos = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .choose_multiple(&mut rng, mines + 1);
        if let Some(safe_pos) = self.safe_pos {
            if let Some(p) = mines_pos.iter().position(|&p| p == safe_pos) {
                mines_pos.remove(p);
            }
        }
        if mines_pos.len() > mines {
            mines_pos.shuffle(&mut rng);
            mines_pos.pop();
        }
        use CellState::Unopened;
        let mut state = GameState {
            options: self,
            mines: (0..h).map(|_| (0..w).map(|_| false).collect()).collect(),
            cells: (0..h).map(|_| (0..w).map(|_| Unopened).collect()).collect(),
        };
        for (x, y) in mines_pos {
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
    pub options: GameOptions,
    pub mines: Vec<Vec<bool>>,
    cells: Vec<Vec<CellState>>,
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

    pub fn cell(&self, x: usize, y: usize) -> CellState {
        self.cells[y][x]
    }

    pub fn set_cell(&mut self, x: usize, y: usize, state: CellState) {
        self.cells[y][x] = state;
    }

    pub fn nearby_cells(&self, x: usize, y: usize) -> Vec<(usize, usize)> {
        let x = x as i32;
        let y = y as i32;
        [y - 1, y, y + 1]
            .iter()
            .flat_map(|y1| {
                let y1 = *y1 as i32;
                if y1 < 0 || y1 >= self.height() as i32 {
                    return [].into();
                }
                [x - 1, x, x + 1]
                    .iter()
                    .filter_map(|x1| {
                        let x1 = *x1 as i32;
                        if x1 < 0 || x1 >= self.width() as i32 {
                            None
                        } else if !(x1 == x && y1 == y) {
                            Some((x1 as usize, y1 as usize))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    pub fn nearby_mines(&self, x: usize, y: usize) -> u8 {
        self.nearby_cells(x, y)
            .into_iter()
            .filter(|(x, y)| self.is_mine(*x, *y))
            .count() as u8
    }

    pub fn is_flag(&self, x: usize, y: usize) -> bool {
        self.cell(x, y) == CellState::Flagged
    }

    pub fn nearby_flags(&self, x: usize, y: usize) -> u8 {
        self.nearby_cells(x, y)
            .into_iter()
            .filter(|(x, y)| self.is_flag(*x, *y))
            .count() as u8
    }

    pub fn is_opened(&self, x: usize, y: usize) -> bool {
        self.cell(x, y) == CellState::Opened
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

impl CellView {
    fn is_intact(&self) -> bool {
        use CellView::*;
        match self {
            Unopened => true,
            Hovered => true,
            Pushed => true,
            _ => false,
        }
    }
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
    state: GameState,
    cells: Vec<Vec<CellView>>,
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

#[derive(Debug, Clone, Default)]
pub struct RedrawCells(pub Vec<(usize, usize)>);

impl RedrawCells {
    pub fn redraw_all(w: usize, h: usize) -> Self {
        Self(
            (0..h)
                .flat_map(|y| (0..w).map(|x| (x, y)).collect::<Vec<_>>())
                .collect(),
        )
    }
}

impl Deref for RedrawCells {
    type Target = Vec<(usize, usize)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RedrawCells {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl GameView {
    pub fn options(&self) -> GameOptions {
        self.state.options.clone()
    }

    pub fn width(&self) -> usize {
        self.state.width()
    }

    pub fn height(&self) -> usize {
        self.state.height()
    }

    pub fn cell(&self, x: usize, y: usize) -> CellView {
        self.cells[y][x]
    }

    pub fn set_cell(&mut self, x: usize, y: usize, cell: CellView) {
        self.cells[y][x] = cell;
    }

    pub fn nearby_cells(&self, x: usize, y: usize) -> Vec<(usize, usize)> {
        self.state.nearby_cells(x, y)
    }

    fn refresh_game_result(&mut self) {
        self.result = self.state.game_result();
        if self.result == GameResult::Win {
            self.flags = self.mines;
        }
    }

    fn refresh_all_cell(&mut self) -> RedrawCells {
        let mut redraw = Vec::new();
        for y in 0..self.state.height() {
            for x in 0..self.state.width() {
                redraw.extend(self.refresh_cell(x, y).0);
            }
        }
        RedrawCells(redraw)
    }

    fn refresh_3x3_cell(&mut self, x: usize, y: usize) -> RedrawCells {
        let mut redraw = Vec::new();
        redraw.extend(self.refresh_cell(x, y).0);
        for (x, y) in self.nearby_cells(x, y) {
            redraw.extend(self.refresh_cell(x, y).0);
        }
        RedrawCells(redraw)
    }

    fn refresh_gesture(&mut self, gesture: Gesture) -> RedrawCells {
        match gesture {
            Gesture::Hover(x, y) | Gesture::LeftOrRightPush(x, y) => self.refresh_cell(x, y),
            Gesture::MidPush(x, y) => self.refresh_3x3_cell(x, y),
            Gesture::None => Default::default(),
        }
    }

    fn refresh_cell(&mut self, x: usize, y: usize) -> RedrawCells {
        use CellView::*;
        use GameResult::*;
        let previous_cell_view = self.cell(x, y);
        let cell_view = match (self.result, self.state.is_mine(x, y), self.state.cell(x, y)) {
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
        self.set_cell(x, y, cell_view);
        if previous_cell_view != cell_view {
            RedrawCells(vec![(x, y)])
        } else {
            Default::default()
        }
    }

    pub fn left_click(&mut self, x: usize, y: usize) -> RedrawCells {
        let mut redraw = Vec::new();
        if self.result != GameResult::Playing {
            return Default::default();
        }
        use CellState::*;
        if self.state.cell(x, y) != Unopened {
            return Default::default();
        }
        if self.state.is_mine(x, y) {
            self.state.set_cell(x, y, Opened);
        } else {
            let mut cells_to_left_click = BTreeSet::new();
            cells_to_left_click.insert((x, y));
            while let Some(cell) = cells_to_left_click.pop_first() {
                let (x, y) = cell;
                if self.state.cell(x, y) == Unopened {
                    self.state.set_cell(x, y, Opened);
                    redraw.extend(self.refresh_cell(x, y).0);
                    if self.state.nearby_mines(x, y) == 0 {
                        for (x, y) in self.nearby_cells(x, y) {
                            cells_to_left_click.insert((x, y));
                        }
                    }
                }
            }
        }
        self.refresh_game_result();
        if self.result != GameResult::Playing {
            redraw.extend(self.refresh_all_cell().0)
        }
        RedrawCells(redraw)
    }

    pub fn right_click(&mut self, x: usize, y: usize) -> RedrawCells {
        if self.result != GameResult::Playing {
            return Default::default();
        }
        use CellState::*;
        let cell_state = self.state.cell(x, y);
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
            Opened => return Default::default(),
        };
        self.state.set_cell(x, y, new_cell_state);
        self.refresh_cell(x, y)
    }

    pub fn middle_click(&mut self, x: usize, y: usize) -> RedrawCells {
        if self.result != GameResult::Playing {
            return Default::default();
        }
        use CellState::*;
        if self.state.cell(x, y) != Opened
            || self.state.nearby_mines(x, y) != self.state.nearby_flags(x, y)
        {
            return Default::default();
        }
        let mut redraw = Vec::new();
        for (x, y) in self.nearby_cells(x, y) {
            if self.state.cell(x, y) == Unopened {
                if (!self.state.is_mine(x, y)) && self.state.nearby_mines(x, y) == 0 {
                    redraw.extend(self.left_click(x, y).0);
                } else {
                    self.state.set_cell(x, y, Opened);
                }
            }
        }
        self.refresh_game_result();
        if self.result != GameResult::Playing {
            redraw.extend(self.refresh_all_cell().0)
        } else {
            redraw.extend(self.refresh_3x3_cell(x as usize, y as usize).0)
        }
        RedrawCells(redraw)
    }

    pub fn gesture(&mut self, gesture: Gesture) -> RedrawCells {
        let previous_gesture = self.gesture;
        self.gesture = gesture;
        let mut redraw = self.refresh_gesture(previous_gesture);
        redraw.0.extend(self.refresh_gesture(gesture).0);
        redraw
    }

    pub fn is_draggable(&self, x: usize, y: usize) -> bool {
        match self.result {
            GameResult::Win | GameResult::Lose => true,
            GameResult::Playing => matches!(
                self.cell(x, y),
                CellView::Opened(_) | CellView::Flagged | CellView::Questioned
            ),
        }
    }

    pub fn automation_step(&mut self) -> Option<RedrawCells> {
        let SolveResult {
            must_be_mine,
            must_not_mine,
        } = self.solve();
        if must_be_mine.is_empty() && must_not_mine.is_empty() {
            return None;
        }
        let mut redraw = HashSet::<(usize, usize)>::new();
        for (x, y) in must_be_mine {
            // TODO: detect human interference
            redraw.extend(self.right_click(x, y).0);
        }
        for (x, y) in must_not_mine {
            redraw.extend(self.left_click(x, y).0);
        }
        for y in 0..self.height() {
            for x in 0..self.width() {
                redraw.extend(self.middle_click(x, y).0);
            }
        }
        Some(RedrawCells(redraw.into_iter().collect()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_game() {
        let options = GameOptions {
            difficulty: Difficulty::Custom {
                width: 3,
                height: 3,
                mines: 3,
            },
            safe_pos: None,
            seed: Some(1),
        };
        let state = options.clone().build();
        assert_eq!(
            state,
            GameState {
                options,
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
                difficulty: Difficulty::Custom {
                    width: 3,
                    height: 3,
                    mines: 3,
                },
                safe_pos: None,
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
