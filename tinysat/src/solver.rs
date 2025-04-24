use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    ops::{Deref, DerefMut},
};

use crate::{Clause, Cnf, Literal, Polarity, Variable};
use Polarity::*;

#[derive(Debug, Clone)]
pub enum Model {
    Satisfied(Assignment),
    Unsatisfiable,
}

impl Model {
    pub fn is_unsat(&self) -> bool {
        matches!(self, Model::Unsatisfiable)
    }
}

impl Display for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Model::Satisfied(assignment) => {
                write!(f, "sat \\left( ")?;
                let mut iter = assignment.iter();
                if let Some((v, a)) = iter.next() {
                    write!(f, "{v} = {a}")?;
                    for (v, a) in iter {
                        write!(f, ", {v} = {a}")?;
                    }
                }
                write!(f, " \\right)")
            }
            Model::Unsatisfiable => write!(f, "unsat"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Assignment(HashMap<Variable, Polarity>);

impl Deref for Assignment {
    type Target = HashMap<Variable, Polarity>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Assignment {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

enum AssignResult {
    Reduced(Cnf),
    Unchanged(Cnf),
}

fn assign(cnf: Cnf, assignment: &Assignment) -> AssignResult {
    let mut reduced = false;
    let cnf = Cnf(cnf
        .0
        .into_iter()
        .filter_map(|clause| {
            let mut new_clause = Vec::new();
            for Literal { variable, polarity } in clause.0 {
                match assignment.get(&variable) {
                    Some(a) => {
                        reduced = true;
                        if *a == polarity {
                            return None;
                        } else {
                            continue;
                        }
                    }
                    None => {
                        new_clause.push(Literal { variable, polarity });
                    }
                }
            }
            Some(Clause(new_clause))
        })
        .collect());
    if reduced {
        AssignResult::Reduced(cnf)
    } else {
        AssignResult::Unchanged(cnf)
    }
}

#[derive(Debug, Clone)]
enum UnitPropagationResult {
    Unsatisfiable,
    Continue(Cnf, Assignment),
}

fn unit_propagation(mut cnf: Cnf) -> UnitPropagationResult {
    let mut implies = Assignment(HashMap::new());
    loop {
        for clause in cnf.0.iter() {
            if clause.0.is_empty() {
                return UnitPropagationResult::Unsatisfiable;
            }
            if clause.0.len() == 1 {
                let Literal { variable, polarity } = clause.0[0];
                match implies.get(&variable) {
                    Some(a) => {
                        if *a != polarity {
                            return UnitPropagationResult::Unsatisfiable;
                        }
                    }
                    None => {
                        implies.insert(variable, polarity);
                    }
                }
            }
        }
        match assign(cnf, &implies) {
            AssignResult::Reduced(new_cnf) => {
                cnf = new_cnf;
                continue;
            }
            AssignResult::Unchanged(new_cnf) => {
                cnf = new_cnf;
                break;
            }
        }
    }
    UnitPropagationResult::Continue(cnf, implies)
}

fn solve_rec(cnf: Cnf, mut variables: HashSet<Variable>) -> Model {
    if cnf.0.is_empty() {
        return Model::Satisfied(Assignment(HashMap::new()));
    }
    let victim = *variables.iter().take(1).collect::<Vec<_>>()[0];
    variables.remove(&victim);
    let AssignResult::Reduced(new_cnf) = assign(
        cnf.clone(),
        &Assignment(HashMap::from_iter([(victim, Positive)])),
    ) else {
        unreachable!();
    };
    match unit_propagation(new_cnf) {
        UnitPropagationResult::Unsatisfiable => {}
        UnitPropagationResult::Continue(cnf, implies) => {
            let mut variables = variables.clone();
            for v in implies.keys() {
                variables.remove(v);
            }
            match solve_rec(cnf, variables) {
                Model::Satisfied(mut assignment) => {
                    assignment.insert(victim, Positive);
                    assignment.extend(implies.0);
                    return Model::Satisfied(assignment);
                }
                Model::Unsatisfiable => {}
            }
        }
    }
    let AssignResult::Reduced(new_cnf) = assign(
        cnf.clone(),
        &Assignment(HashMap::from_iter([(victim, Negative)])),
    ) else {
        unreachable!();
    };
    match unit_propagation(new_cnf) {
        UnitPropagationResult::Unsatisfiable => {}
        UnitPropagationResult::Continue(cnf, implies) => {
            let mut variables = variables.clone();
            for v in implies.keys() {
                variables.remove(v);
            }
            match solve(cnf) {
                Model::Satisfied(mut assignment) => {
                    assignment.insert(victim, Negative);
                    assignment.extend(implies.0);
                    return Model::Satisfied(assignment);
                }
                Model::Unsatisfiable => {}
            }
        }
    }
    Model::Unsatisfiable
}

pub fn solve(cnf: Cnf) -> Model {
    let variables = cnf
        .0
        .iter()
        .flat_map(|clause| clause.0.iter().map(|Literal { variable, .. }| *variable))
        .collect();
    solve_rec(cnf, variables)
}
