pub fn tmp_json_path() -> std::path::PathBuf {
    use rand::distributions::{Alphanumeric, DistString};

    const DIR: &str = "/tmp/";
    const EXT: &str = ".json";
    const LEN: usize = 16;

    let mut path = String::with_capacity(DIR.len() + LEN + EXT.len());
    path.push_str(DIR);
    Alphanumeric.append_string(&mut rand::thread_rng(), &mut path, LEN);
    path.push_str(EXT);
    path.into()
}
