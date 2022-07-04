use ariadne::Source;
use chumsky::prelude::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::Range};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum Value {
	Null,
	String(String),
	Number(f64),
	Range(i64, i64),
	Boolean(bool),
}

impl std::fmt::Display for Value {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Value::Null => Ok(()),
			Value::String(v) => write!(f, "{}", v),
			Value::Number(v) => write!(f, "{}", v),
			Value::Boolean(v) => write!(f, "{}", v),
			Value::Range(start, end) => write!(f, "{}..{}", start, end),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum Node {
	NumericLiteral(f64),
	StringLiteral(String),
	BooleanLiteral(bool),
	Ident(String),
	Op(String),
	Call(Box<Spanned>, Vec<Spanned>),
	While(Box<Spanned>, Vec<Spanned>),
	Range(i64, i64),
	Variable(String, Box<Spanned>),
}

pub type Spanned = (Node, Range<usize>);

// A parser that turns pythonic code with semantic whitespace into a token tree
pub fn lexer() -> impl Parser<char, Vec<Spanned>, Error = Simple<char>> {
	let number = text::int(10)
		.separated_by(just('.'))
		.labelled("number")
		.try_map(|v, span| {
			Ok(Node::NumericLiteral(
				v.join(".")
					.parse::<f64>()
					.map_err(|e| Simple::custom(span, format!("{}", e)))?,
			))
		});

	let range = text::int(10).separated_by(just("..")).try_map(
		|v: Vec<String>, span: Range<usize>| {
			let v1 = v.get(0);
			let v2 = v.get(1);

			if v1.is_none() || v2.is_none() {
				return Err(Simple::custom(span, "invalid range"));
			}

			Ok(Node::Range(
				v1.unwrap().parse::<i64>().map_err(|e| {
					Simple::custom(span.clone(), format!("{}", e))
				})?,
				v2.unwrap()
					.parse::<i64>()
					.map_err(|e| Simple::custom(span, format!("{}", e)))?,
			))
		},
	);

	let string = just('`')
		.ignore_then(filter(|c| *c != '\\' && *c != '`').repeated())
		.then_ignore(just('`'))
		.labelled("string")
		.collect::<String>()
		.map(Node::StringLiteral);

	let boolean = just("true")
		.or(just("false"))
		.map(|b| Node::BooleanLiteral(b == "true"));

	let ident = text::ident()
		.labelled("identifier")
		.map_with_span(|ident, span| (Node::Ident(ident), span));

	let op = one_of("=.:%,")
		.repeated()
		.at_least(1)
		.collect()
		.labelled("operator")
		.map(Node::Op);

	let tt = recursive(|tt| {
		let tt_span = tt.clone().padded().map_with_span(|n, span| (n, span));

		let func_call = text::ident()
			.map_with_span(|name, span| (Node::Ident(name), span))
			.then(
				ident
					.or(tt_span.clone())
					.padded()
					.separated_by(just(','))
					.allow_trailing(),
			)
			.labelled("function call")
			.map(|(name, args)| Node::Call(Box::new(name), args));

		let block = tt_span
			.clone()
			.padded()
			.then_ignore(just('.').or(just(';')).or_not())
			.repeated()
			.or_not()
			.delimited_by(just('{'), just('}'))
			.labelled("block");

		let variable = text::ident()
			.padded()
			.then_ignore(just(":=").padded())
			.then(tt_span.clone().padded())
			.labelled("variable")
			.map(|(name, value)| Node::Variable(name, Box::new(value)));

		let while_loop = just("while")
			.padded()
			.ignore_then(tt_span)
			.then(block)
			.padded()
			.labelled("while loop")
			.map(|(condition, body)| {
				Node::While(Box::new(condition), body.unwrap_or_default())
			});

		while_loop
			.or(boolean)
			.or(string)
			.or(range)
			.or(number)
			.or(variable)
			.or(op)
			.or(func_call)
	})
	.map_with_span(|n, span| (n, span));

	tt.padded()
		.then_ignore(just('.').or(just(';')).or_not())
		.repeated()
		.then_ignore(end())
}

use thiserror::Error;

#[derive(Debug, Error)]
pub enum EvaluationError {
	#[error("function {0} not found")]
	FunctionNotFound(String),
	#[error("variable {0} not found")]
	VariableNotFound(String),
}

pub type SpannedEvaluationError = (EvaluationError, Range<usize>);

pub fn eval(
	source: &Source,
	node: &Spanned,
	variables: &mut HashMap<String, Value>,
) -> Result<Value, SpannedEvaluationError> {
	match &node.0 {
		Node::Call(cname, args) => {
			if let Node::Ident(name) = cname.0.clone() {
				if name.as_str() == "display" {
					let mut result = String::new();

					for arg in args {
						let value = eval(source, arg, variables)?;

						result += &value.to_string();
					}

					println!("{}", result);
				} else {
					return Err((
						EvaluationError::FunctionNotFound(name),
						cname.1.clone(),
					));
				}
			}
		}
		Node::StringLiteral(s) => return Ok(Value::String(s.clone())),
		Node::NumericLiteral(n) => return Ok(Value::Number(*n)),
		Node::BooleanLiteral(b) => return Ok(Value::Boolean(*b)),
		Node::Range(start, end) => return Ok(Value::Range(*start, *end)),
		Node::While(cond, body) => {
			let condition = eval(source, cond, variables)?;

			match condition {
				Value::Number(n) => {
					if n > 0.0 {
						loop {
							for node in body {
								eval(source, node, variables)?;
							}
						}
					}
				}
				Value::Boolean(bool) => {
					if bool {
						loop {
							for node in body {
								eval(source, node, variables)?;
							}
						}
					}
				}
				Value::Range(start, end) => {
					for _ in start..end {
						for node in body {
							eval(source, node, variables)?;
						}
					}
				}
				_ => {}
			}
		}
		Node::Variable(name, value) => {
			let val = eval(source, value, variables)?;

			variables.insert(name.to_string(), val);
		}
		Node::Ident(ident) => {
			let var = variables.get(ident);

			if let Some(var) = var {
				return Ok(var.clone());
			} else {
				return Err((
					EvaluationError::VariableNotFound(ident.to_string()),
					node.1.clone(),
				));
			}
		}
		n => panic!("not implemented: {:?}", n),
	}

	Ok(Value::Null)
}
