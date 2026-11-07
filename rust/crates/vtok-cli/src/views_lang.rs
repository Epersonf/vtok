//! Dedicated views for the JS / Python / Ruby / PHP ecosystems (see cmdviews.rs for the contract).
//! `view(cmd, lines)` decides the tool from the command line and returns None when the output is not
//! what that tool prints.
mod js_build;
mod json_lint;
mod json_test;
mod json_util;
#[cfg(test)] mod t_json;
mod js_lint;
mod js_pkg;
mod js_playwright;
mod js_test;
mod js_tsc;
mod php;
mod php_static;
mod ts_tsc;
#[cfg(test)] mod t_tsc;
mod py;
mod py_pkg;
mod py_pytest;
mod py_pytest_fmt;
mod py_pytest_items;
#[cfg(test)] mod t_pytest;
#[cfg(test)] mod t_pytest_modes;
mod rb;
mod util;

/// pytest's FAILURES report by shape, for runs whose command line does not name pytest (`make test`, CI scripts).
pub fn pytest_view(lines: &[String]) -> Option<String> { py::pytest(lines) }

pub fn view(cmd: &str, lines: &[String]) -> Option<String> {
    let c = util::parse(cmd)?;
    let a = &c.args;
    let l = lines;
    let has = |s: &str| a.iter().any(|x| x == s);
    let first = a.first().map(|s| s.as_str()).unwrap_or("");
    match c.tool.as_str() {
        "vitest" => js_test::vitest(l),
        "jest" => js_test::jest(l),
        "playwright" => json_test::playwright(l).or_else(|| js_playwright::playwright(l)),
        "prettier" => js_tsc::prettier(l),
        "tsc" | "vue-tsc" => ts_tsc::tsc(l),
        "eslint" => json_lint::eslint(l).or_else(|| js_lint::eslint(l)),
        "oxlint" => js_lint::oxlint(l),
        "biome" => js_lint::biome(l),
        "next" if first == "build" => js_build::next(l),
        "ng" if first == "build" || first == "b" => js_build::ng(l),
        "prisma" => js_build::prisma(l),
        "npm" | "pnpm" | "yarn" | "bun" | "deno" => js_pkg::pkg(&c.tool, a, l),
        "mypy" | "dmypy" => py::mypy(l),
        "ruff" if has("format") => py::ruff_format(l),
        "ruff" => py::ruff_check(l),
        "pytest" | "py.test" => py::pytest(l),
        "pip" | "pip3" => py_pkg::pip(a, l),
        "uv" => py_pkg::uv(a, l),
        "sqlfluff" => json_lint::sqlfluff(l).or_else(|| py_pkg::sqlfluff(l)),
        "rspec" => json_test::rspec(l).or_else(|| rb::rspec(l)),
        "rubocop" => json_lint::rubocop(l).or_else(|| rb::rubocop(l)),
        "rake" | "rails" | "ruby" => rb::minitest(l),
        "phpunit" | "pest" | "paratest" => php::tests(l),
        "artisan" | "sail" if has("test") => php::tests(l),
        "phpstan" => json_lint::phpstan(l).or_else(|| php_static::table(l, false)),
        "ecs" => php_static::table(l, true),
        "pint" => php_static::pint(l),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::util::parse;

    #[test]
    fn parses_prefixes() {
        let c = parse("cd app && CI=1 npx --yes vitest run src").unwrap();
        assert_eq!((c.tool.as_str(), c.args[0].as_str()), ("vitest", "run"));
        assert_eq!(parse("pnpm exec eslint .").unwrap().tool, "eslint");
        assert_eq!(parse("pnpm vitest").unwrap().tool, "vitest");
        assert_eq!(parse("uv run python -m pytest -x").unwrap().tool, "pytest");
        assert_eq!(parse("bundle exec rspec").unwrap().tool, "rspec");
        assert_eq!(parse("php vendor/bin/phpunit --testdox").unwrap().tool, "phpunit");
        assert_eq!(parse("php artisan test").unwrap().tool, "artisan");
        assert_eq!(parse("./node_modules/.bin/tsc.cmd --noEmit").unwrap().tool, "tsc");
    }

    #[test]
    fn unknown_tool_is_none() {
        assert!(super::view("ls -la", &["a".to_string()]).is_none());
    }
}
