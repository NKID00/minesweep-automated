mod solver;
use solver::solve;
pub use solver::Model;

use std::{
    fmt::Display,
    ops::{BitAnd, BitOr, BitXor},
};

#[derive(Debug, Clone)]
pub enum Formula {
    Variable(Variable),
    Negation(Box<Formula>),
    Conjunction(Box<Formula>, Box<Formula>),
    Disjunction(Box<Formula>, Box<Formula>),
    Equivalence(Box<Formula>, Box<Formula>),
    Implication(Box<Formula>, Box<Formula>),
}

impl Display for Formula {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Formula::{Conjunction, Disjunction, Equivalence, Implication, Negation};
        match self {
            Formula::Variable(v) => write!(f, "{v}"),
            Negation(f0) => match f0.encode_literal() {
                Some(_) => write!(f, "\\lnot {f0}"),
                None => write!(f, "\\lnot \\left( {f0} \\right)"),
            },
            Conjunction(f0, f1)
            | Disjunction(f0, f1)
            | Equivalence(f0, f1)
            | Implication(f0, f1) => {
                let f0 = match f0.encode_literal() {
                    Some(_) => f0.to_string(),
                    None => format!("\\left( {f0} \\right)"),
                };
                let f1 = match f1.encode_literal() {
                    Some(_) => f1.to_string(),
                    None => format!("\\left( {f1} \\right)"),
                };
                match self {
                    Conjunction(_, _) => {
                        write!(f, "{f0} \\land {f1}")
                    }
                    Disjunction(_, _) => {
                        write!(f, "{f0} \\lor {f1}")
                    }
                    Equivalence(_, _) => {
                        write!(f, "{f0} \\leftrightarrow {f1}")
                    }
                    Implication(_, _) => {
                        write!(f, "{f0} \\to {f1}")
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}

impl Formula {
    pub fn maximum_variable(&self) -> Variable {
        use Formula::{Conjunction, Disjunction, Equivalence, Implication, Negation};
        let mut ans: Variable = 0.into();
        let mut formulas = vec![self];
        while let Some(f) = formulas.pop() {
            match f {
                Formula::Variable(v) => ans = ans.max(*v),
                Negation(f) => formulas.push(f),
                Conjunction(f0, f1) => {
                    formulas.push(f0);
                    formulas.push(f1);
                }
                Disjunction(f0, f1) => {
                    formulas.push(f0);
                    formulas.push(f1);
                }
                Equivalence(f0, f1) => {
                    formulas.push(f0);
                    formulas.push(f1);
                }
                Implication(f0, f1) => {
                    formulas.push(f0);
                    formulas.push(f1);
                }
            }
        }
        ans
    }

    fn combine_negation(&self) -> (Polarity, &Self) {
        use Formula::*;
        use Polarity::*;
        match self {
            Negation(f) => {
                let (p, f) = f.combine_negation();
                (p.negate(), f)
            }
            f => (Positive, f),
        }
    }

    fn encode_literal(&self) -> Option<Literal> {
        use Formula::*;
        match self {
            Variable(v) => Some(Literal::positive(*v)),
            Negation(f) => f.encode_literal().map(|l| l.negate()),
            _ => None,
        }
    }

    pub fn tseitin_encode(&self) -> Cnf {
        use Formula::{Conjunction, Disjunction, Equivalence, Implication, Negation};
        if let Some(l) = self.encode_literal() {
            return Cnf(vec![Clause(vec![l])]);
        }
        let mut max_var = self.maximum_variable();
        let mut subformulas: Vec<(Literal, &Formula)> = vec![];
        let mut clauses = vec![];
        fn wrap_formula<'a>(
            f: &'a Formula,
            max_var: &mut Variable,
            subformulas: &mut Vec<(Literal, &'a Formula)>,
        ) -> Literal {
            use Polarity::*;
            match f.encode_literal() {
                Some(l) => l,
                None => match f.combine_negation() {
                    (Positive, f) => {
                        *max_var = max_var.next_variable();
                        subformulas.push((Literal::positive(*max_var), f));
                        Literal::positive(*max_var)
                    }
                    (Negative, f) => {
                        *max_var = max_var.next_variable();
                        subformulas.push((Literal::positive(*max_var), f));
                        Literal::negative(*max_var)
                    }
                },
            }
        }
        let l = wrap_formula(self, &mut max_var, &mut subformulas);
        clauses.push(Clause(vec![l]));
        while let Some((v, f)) = subformulas.pop() {
            match f {
                Formula::Variable(_) | Negation(_) => unreachable!(),
                Conjunction(f0, f1) => {
                    let f0_literal = wrap_formula(f0, &mut max_var, &mut subformulas);
                    let f1_literal = wrap_formula(f1, &mut max_var, &mut subformulas);
                    clauses.extend([
                        Clause(vec![v, f0_literal.negate(), f1_literal.negate()]),
                        Clause(vec![v.negate(), f0_literal]),
                        Clause(vec![v.negate(), f1_literal]),
                    ]);
                }
                Disjunction(f0, f1) => {
                    let f0_literal = wrap_formula(f0, &mut max_var, &mut subformulas);
                    let f1_literal = wrap_formula(f1, &mut max_var, &mut subformulas);
                    clauses.extend([
                        Clause(vec![v.negate(), f0_literal, f1_literal]),
                        Clause(vec![v, f0_literal.negate()]),
                        Clause(vec![v, f1_literal.negate()]),
                    ]);
                }
                Equivalence(f0, f1) => {
                    let f0_literal = wrap_formula(f0, &mut max_var, &mut subformulas);
                    let f1_literal = wrap_formula(f1, &mut max_var, &mut subformulas);
                    clauses.extend([
                        Clause(vec![v, f0_literal.negate(), f1_literal.negate()]),
                        Clause(vec![v, f0_literal, f1_literal]),
                        Clause(vec![v.negate(), f0_literal.negate(), f1_literal]),
                        Clause(vec![v.negate(), f0_literal, f1_literal.negate()]),
                    ]);
                }
                Implication(f0, f1) => {
                    let f0_literal = wrap_formula(f0, &mut max_var, &mut subformulas);
                    let f1_literal = wrap_formula(f1, &mut max_var, &mut subformulas);
                    clauses.extend([
                        Clause(vec![v, f0_literal, f1_literal]),
                        Clause(vec![v.negate(), f0_literal.negate(), f1_literal]),
                        Clause(vec![v, f1_literal.negate()]),
                    ]);
                }
            }
        }
        Cnf(clauses)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cnf(Vec<Clause>);

impl Cnf {
    pub fn solve(&self) -> Model {
        solve(self.clone())
    }
}

impl Display for Cnf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            write!(f, "\\bf T")
        } else {
            write!(f, "{}", self.0[0])?;
            for l in self.0.iter().skip(1) {
                write!(f, " \\land {}", l)?;
            }
            Ok(())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clause(Vec<Literal>);

impl Display for Clause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            write!(f, "\\bf F")
        } else {
            write!(f, "\\left( {}", self.0[0])?;
            for l in self.0.iter().skip(1) {
                write!(f, " \\lor {}", l)?;
            }
            write!(f, " \\right)")
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Literal {
    variable: Variable,
    polarity: Polarity,
}

impl Literal {
    fn positive(variable: Variable) -> Self {
        Literal {
            variable,
            polarity: Polarity::Positive,
        }
    }

    fn negative(variable: Variable) -> Self {
        Literal {
            variable,
            polarity: Polarity::Negative,
        }
    }

    fn negate(&self) -> Self {
        Self {
            polarity: self.polarity.negate(),
            ..*self
        }
    }
}

impl Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Polarity::*;
        match self.polarity {
            Positive => write!(f, "{}", self.variable),
            Negative => write!(f, "\\overline{{{}}}", self.variable),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Variable(pub usize);

impl Variable {
    fn next_variable(&self) -> Self {
        Self(self.0 + 1)
    }
}

impl Display for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "x_{{{}}}", self.0)
    }
}

impl From<usize> for Variable {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Polarity {
    Positive,
    Negative,
}

impl Polarity {
    fn negate(&self) -> Polarity {
        use Polarity::*;
        match self {
            Positive => Negative,
            Negative => Positive,
        }
    }
}

impl BitAnd for Polarity {
    type Output = Polarity;

    fn bitand(self, rhs: Self) -> Self::Output {
        use Polarity::*;
        match (self, rhs) {
            (Positive, Positive) => Positive,
            _ => Negative,
        }
    }
}

impl BitOr for Polarity {
    type Output = Polarity;

    fn bitor(self, rhs: Self) -> Self::Output {
        use Polarity::*;
        match (self, rhs) {
            (Negative, Negative) => Negative,
            _ => Positive,
        }
    }
}

impl BitXor for Polarity {
    type Output = Polarity;

    fn bitxor(self, rhs: Self) -> Self::Output {
        use Polarity::*;
        if self != rhs {
            Positive
        } else {
            Negative
        }
    }
}

impl Display for Polarity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Polarity::*;
        match self {
            Positive => write!(f, "1"),
            Negative => write!(f, "0"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_formula() -> Formula {
        use Formula::*;
        Disjunction(
            Box::new(Implication(
                Box::new(Variable(1.into())),
                Box::new(Conjunction(
                    Box::new(Variable(3.into())),
                    Box::new(Variable(4.into())),
                )),
            )),
            Box::new(Implication(
                Box::new(Variable(2.into())),
                Box::new(Conjunction(
                    Box::new(Variable(3.into())),
                    Box::new(Variable(5.into())),
                )),
            )),
        )
    }

    #[test]
    fn formula() {
        let f = default_formula();
        println!("{f}");
        assert_eq!(f.maximum_variable(), 5.into());
        let cnf = f.tseitin_encode();
        println!("{cnf}");
        let model = cnf.solve();
        println!("{model}");
    }
}
