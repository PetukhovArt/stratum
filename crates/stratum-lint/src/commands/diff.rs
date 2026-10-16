use camino::Utf8Path;

pub fn run(_root: &Utf8Path, _base: &str, _config_path: Option<&Utf8Path>) -> miette::Result<i32> {
    Err(miette::miette!("stratum-lint diff is not implemented yet"))
}
