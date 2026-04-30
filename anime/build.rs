use std::{
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};

use anyhow::Context;
use walkdir::WalkDir;

fn main() -> anyhow::Result<()> {
    const ROOT: &str = env!("CARGO_MANIFEST_DIR");

    println!("cargo:rerun-if-changed=graphql/");

    minify_graphql_queries(
        &PathBuf::from_iter([ROOT, "graphql/"]),
        &PathBuf::from_iter([ROOT, "generated/graphql"]),
    )?;
    Ok(())
}

fn minify_graphql_queries(sources: &Path, dest: &Path) -> anyhow::Result<()> {
    for entry in WalkDir::new(sources) {
        let entry = entry.context("failed to read directory entry")?;

        if !entry.file_type().is_file() {
            continue;
        }

        let full_path = entry.path();

        if full_path
            .extension()
            .is_some_and(|e| e != OsStr::new("gql"))
        {
            continue;
        }

        let relative_path = full_path.strip_prefix(sources).unwrap_or(full_path);
        let new_path = PathBuf::from_iter([dest, relative_path]);

        if let Some(new_parent) = new_path.parent() {
            match fs::create_dir_all(new_parent) {
                Ok(_) => (),
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => (),
                err @ Err(_) => {
                    return err.context(format!(
                        "failed to create destination directory at `{}`",
                        dest.display()
                    ));
                }
            }
        }

        let contents = fs::read_to_string(full_path)
            .with_context(|| format!("failed to read `{}`", full_path.display()))?;

        fs::write(&new_path, minimize_graphql(&contents)).with_context(|| {
            format!("failed to write minified file to `{}`", new_path.display())
        })?;
    }

    Ok(())
}

fn minimize_graphql(query: &str) -> String {
    let mut new = String::with_capacity(query.len());
    let mut had_recent_newline = false;
    let mut had_recent_bracket = false;

    for ch in query.chars() {
        match ch {
            ' ' | '\t' | '\r' => (),
            '{' | '}' => {
                had_recent_newline = false;
                had_recent_bracket = true;
                new.push(ch);
            }
            '\n' => had_recent_newline = true,
            _ => {
                if had_recent_newline {
                    if !had_recent_bracket {
                        // a newline without a bracket likely indicates a field, so separate it
                        new.push(',');
                    }

                    had_recent_newline = false;
                    had_recent_bracket = false;
                }

                new.push(ch);
            }
        }
    }

    new
}
