//! Python bindings. The friendly API lives in python/vtok/__init__.py; this module is the thin native layer.
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::collections::{BTreeMap, HashMap};
use vtok_api as api;

fn err(e: api::VtokError) -> PyErr { PyValueError::new_err(e.to_string()) }

type Table = HashMap<String, String>;

#[pyfunction]
fn languages() -> Vec<(String, Vec<String>)> {
    api::languages().iter().map(|l| (l.name().to_string(), l.extensions().iter().map(|e| e.to_string()).collect())).collect()
}

#[pyfunction]
fn grammar(lang: &str) -> PyResult<String> { Ok(api::language(lang).map_err(err)?.grammar_prompt().to_string()) }

#[pyfunction]
fn ntok(text: &str) -> usize { api::ntok(text) }

#[pyfunction]
#[pyo3(signature = (lang, source, alias=true, view="full", hide_literals=0, table=None, verify=true))]
fn encode(lang: &str, source: &str, alias: bool, view: &str, hide_literals: usize, table: Option<Table>, verify: bool) -> PyResult<(String, Table, Table)> {
    let o = api::Options {
        alias: Some(alias), view: Some(view.to_string()), hide_literals: Some(hide_literals), verify: Some(verify),
        table: table.map(|m| m.into_iter().collect::<BTreeMap<_, _>>()),
    };
    let r = api::encode(lang, source, &o).map_err(err)?;
    Ok((r.vtok, r.table.into_iter().collect(), r.literals.into_iter().collect()))
}

#[pyfunction]
#[pyo3(signature = (lang, vtok, table=None, literals=None))]
fn decode(lang: &str, vtok: &str, table: Option<Table>, literals: Option<Table>) -> PyResult<String> {
    let inv: Table = table.unwrap_or_default().into_iter().map(|(o, a)| (a, o)).collect();
    api::decode(lang, vtok, &inv, &literals.unwrap_or_default()).map_err(err)
}

#[pyfunction]
#[pyo3(signature = (lang, source, max_lines=150))]
fn split(lang: &str, source: &str, max_lines: usize) -> PyResult<(String, bool, Table, Vec<(String, String, usize, usize)>)> {
    let r = api::split_file(lang, source, max_lines).map_err(err)?;
    Ok((r.index, r.whole_file, r.table.into_iter().collect(),
        r.chunks.into_iter().map(|c| (c.id, c.label, c.start, c.end)).collect()))
}

#[pyfunction]
#[pyo3(signature = (lang, source, id, max_lines=150))]
fn chunk(lang: &str, source: &str, id: &str, max_lines: usize) -> PyResult<String> {
    api::chunk_of(lang, source, id, max_lines).map_err(err)
}

#[pyfunction]
#[pyo3(signature = (lang, source, edits, max_lines=150, force=false))]
fn apply(lang: &str, source: &str, edits: Table, max_lines: usize, force: bool) -> PyResult<String> {
    api::apply_chunks(lang, source, &edits.into_iter().collect(), max_lines, force).map_err(err)
}

#[pyfunction]
fn project_table(lang: &str, sources: Vec<String>) -> PyResult<Table> {
    let refs: Vec<&str> = sources.iter().map(|s| s.as_str()).collect();
    Ok(api::project_table(lang, &refs).map_err(err)?.into_iter().collect())
}

#[pyfunction]
fn run_cli(args: Vec<String>) -> i32 { vtok_cli::run(&args) }

#[pymodule]
fn _vtok(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(languages, m)?)?;
    m.add_function(wrap_pyfunction!(grammar, m)?)?;
    m.add_function(wrap_pyfunction!(ntok, m)?)?;
    m.add_function(wrap_pyfunction!(encode, m)?)?;
    m.add_function(wrap_pyfunction!(decode, m)?)?;
    m.add_function(wrap_pyfunction!(split, m)?)?;
    m.add_function(wrap_pyfunction!(chunk, m)?)?;
    m.add_function(wrap_pyfunction!(apply, m)?)?;
    m.add_function(wrap_pyfunction!(project_table, m)?)?;
    m.add_function(wrap_pyfunction!(run_cli, m)?)?;
    Ok(())
}
