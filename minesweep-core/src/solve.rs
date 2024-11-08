use std::collections::HashSet;

use itertools::Itertools;
use tinysat::{Cnf, Formula, Variable};

use crate::{CellView, GameResult, GameView};

#[derive(Debug, Clone, Default)]
pub struct SolveResult {
    pub must_be_mine: Vec<(usize, usize)>,
    pub must_not_mine: Vec<(usize, usize)>,
}

impl SolveResult {
    fn merge(&mut self, other: SolveResult) {
        self.must_be_mine.extend(other.must_be_mine);
        self.must_not_mine.extend(other.must_not_mine);
    }
}

impl GameView {
    /// Returns a variable such that variable is true iff (x, y) is mine
    fn mine_var(self: &GameView, x: usize, y: usize) -> Variable {
        Variable(y * self.width() + x)
    }

    fn constraint_cell(self: &GameView, x: usize, y: usize) -> Option<Formula> {
        use CellView::*;
        use Formula::*;
        match self.cell(x, y) {
            Flagged => Some(Variable(self.mine_var(x, y))),
            Opened(n) => {
                let nearby_cells = self.nearby_cells(x, y);
                let formula = if n == 0 {
                    nearby_cells
                        .clone()
                        .into_iter()
                        .map(|cell| Negation(Box::new(Variable(self.mine_var(cell.0, cell.1)))))
                        .reduce(|f0, f1| Conjunction(Box::new(f0), Box::new(f1)))
                        .unwrap()
                } else {
                    nearby_cells
                        .clone()
                        .into_iter()
                        .combinations(n as usize)
                        .map(|mines| {
                            nearby_cells
                                .clone()
                                .into_iter()
                                .map(|cell| {
                                    if mines.contains(&cell) {
                                        Variable(self.mine_var(cell.0, cell.1))
                                    } else {
                                        Negation(Box::new(Variable(self.mine_var(cell.0, cell.1))))
                                    }
                                })
                                .reduce(|f0, f1| Conjunction(Box::new(f0), Box::new(f1)))
                                .unwrap()
                        })
                        .reduce(|f0, f1| Disjunction(Box::new(f0), Box::new(f1)))
                        .unwrap()
                };
                Some(Conjunction(
                    Box::new(formula),
                    Box::new(Negation(Box::new(Variable(self.mine_var(x, y))))),
                ))
            }
            _ => None,
        }
    }

    /// Generate constraints known from current view
    fn constraints(self: &GameView, intact_cells_to_examine: &HashSet<(usize, usize)>) -> Formula {
        use Formula::*;
        let mut cells_to_examine: HashSet<(usize, usize)> = HashSet::new();
        for y in 0..self.height() {
            for x in 0..self.width() {
                // for (x, y) in intact_cells_to_examine {
                cells_to_examine.extend(self.nearby_cells(x, y));
                // cells_to_examine.extend(self.nearby_cells(*x, *y));
                // }
            }
        }
        leptos::logging::log!("{cells_to_examine:?}");
        cells_to_examine
            .into_iter()
            .filter_map(|(x, y)| self.constraint_cell(x, y))
            .reduce(|f0, f1| Conjunction(Box::new(f0), Box::new(f1)))
            .unwrap()
    }

    fn check_cell(self: &GameView, constraints: &Cnf, x: usize, y: usize) -> SolveResult {
        use Formula::*;
        let mut assume_is_mine: Cnf = constraints.clone();
        assume_is_mine.merge(Variable(self.mine_var(x, y)).into());
        if assume_is_mine.solve().is_unsat() {
            return SolveResult {
                must_be_mine: vec![],
                must_not_mine: vec![(x, y)],
            };
        }
        let mut assume_not_mine: Cnf = constraints.clone();
        assume_not_mine.merge(Negation(Box::new(Variable(self.mine_var(x, y)))).into());
        if assume_not_mine.solve().is_unsat() {
            return SolveResult {
                must_be_mine: vec![(x, y)],
                must_not_mine: vec![],
            };
        }
        SolveResult::default()
    }

    pub fn solve(self: &GameView) -> SolveResult {
        if self.result != GameResult::Playing {
            return SolveResult::default();
        }
        let mut cells_to_examine = HashSet::new();
        for y in 0..self.height() {
            for x in 0..self.width() {
                match self.cell(x, y) {
                    CellView::Flagged | CellView::Opened(_) => {
                        for (x, y) in self.nearby_cells(x, y) {
                            if self.cell(x, y).is_intact() {
                                cells_to_examine.insert((x, y));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        let constraints = self
            .constraints(&cells_to_examine)
            .tseitin_encode(Variable(0x10000));
        let mut result = SolveResult::default();
        for (x, y) in cells_to_examine {
            result.merge(self.check_cell(&constraints, x, y));
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn simple() {
        let mut view = GameView::from(
            GameOptions {
                difficulty: Difficulty::Custom {
                    width: 5,
                    height: 5,
                    mines: 2,
                },
                safe_pos: None,
                seed: Some(4),
            }
            .build(),
        );
        println!("{view:?}");
        view.left_click(0, 0);
        println!("{view:?}");
        let result = view.solve();
        println!("{result:?}");
    }
}
