use ariadne::{Color, Fmt, Label, Report, ReportKind, Source};
use chumsky::{prelude::Simple, Parser};
use paris_lang::{eval, lexer, Value};
use std::{collections::HashMap, env, fs};

fn main() {
	let src =
		fs::read_to_string(env::args().nth(1).expect("Expected file argument"))
			.expect("Failed to read file");

	let (ast, mut errs) = lexer().parse_recovery(src.as_str());
	let mut variables: HashMap<String, Value> = HashMap::new();

	if let Some(ast) = ast.as_ref() {
		if cfg!(debug_assertions) {
			dbg!(ast);
		}

		for node in ast {
			match eval(&src.clone().into(), node, &mut variables) {
				Ok(val) => print!("{}", val),
				Err(e) => errs.push(Simple::custom(e.1, e.0)),
			}
		}
	}

	errs.into_iter()
		.map(|e| e.map(|c| c.to_string()))
		.for_each(|e| {
			let report = Report::build(ReportKind::Error, (), e.span().start);

			let report = match e.reason() {
				chumsky::error::SimpleReason::Unclosed { span, delimiter } => {
					report
						.with_message(format!(
							"Unclosed delimiter {}",
							delimiter.fg(Color::Yellow)
						))
						.with_label(
							Label::new(span.clone())
								.with_message(format!(
									"Unclosed delimiter {}",
									delimiter.fg(Color::Yellow)
								))
								.with_color(Color::Yellow),
						)
						.with_label(
							Label::new(e.span())
								.with_message(format!(
									"Must be closed before this {}",
									e.found()
										.unwrap_or(&"end of file".to_string())
										.fg(Color::Red)
								))
								.with_color(Color::Red),
						)
				}
				chumsky::error::SimpleReason::Unexpected => report
					.with_message(format!(
						"{}, expected {}",
						if e.found().is_some() {
							"Unexpected token in input"
						} else {
							"Unexpected end of input"
						},
						if e.expected().len() == 0 {
							"something else".to_string()
						} else {
							e.expected()
								.map(|expected| match expected {
									Some(expected) => expected.to_string(),
									None => "end of input".to_string(),
								})
								.collect::<Vec<_>>()
								.join(", ")
						}
					))
					.with_label(
						Label::new(e.span())
							.with_message(format!(
								"Unexpected token {}",
								e.found()
									.unwrap_or(&"end of file".to_string())
									.fg(Color::Red)
							))
							.with_color(Color::Red),
					),
				chumsky::error::SimpleReason::Custom(msg) => {
					report.with_message(msg).with_label(
						Label::new(e.span())
							.with_message(format!("{}", msg.fg(Color::Red)))
							.with_color(Color::Red),
					)
				}
			};

			report.finish().print(Source::from(&src)).unwrap();
		});
}
