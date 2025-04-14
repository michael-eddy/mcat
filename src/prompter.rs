use ignore::WalkBuilder;
use inquire::{Confirm, MultiSelect};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub fn prompt_for_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let recursive = Confirm::new("Search recursively?")
        .with_default(true)
        .prompt()
        .map_err(|e| e.to_string())?;

    let mut all_paths = collect_gitignored_paths(dir, recursive)?;
    all_paths.sort(); // Ensures folders come before contents

    let tree_view = format_file_list(&all_paths, dir);

    let index_map: HashMap<String, PathBuf> = tree_view
        .iter()
        .cloned()
        .zip(all_paths.iter().cloned())
        .collect();

    let selected = MultiSelect::new("Select files or folders", tree_view)
        .with_page_size(20)
        .with_vim_mode(true)
        .prompt()
        .map_err(|e| e.to_string())?;

    let selected_paths: HashSet<PathBuf> = selected
        .into_iter()
        .filter_map(|label| index_map.get(&label).cloned())
        .collect();

    // Avoid duplicates: if a folder is selected, skip its inner files
    let mut final_files = HashSet::new();

    for path in &selected_paths {
        if path.is_file() {
            // Only include files not covered by a selected folder
            let covered = selected_paths
                .iter()
                .any(|other| other.is_dir() && path.starts_with(other));
            if !covered {
                final_files.insert(path.clone());
            }
        } else if path.is_dir() {
            for file in all_paths
                .iter()
                .filter(|p| p.is_file() && p.starts_with(path))
            {
                final_files.insert(file.clone());
            }
        }
    }

    Ok(final_files.into_iter().collect())
}

fn collect_gitignored_paths(dir: &Path, recursive: bool) -> Result<Vec<PathBuf>, String> {
    let walker = WalkBuilder::new(dir)
        .follow_links(true)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .max_depth(if recursive { None } else { Some(1) })
        .build();

    let mut paths = vec![];

    for result in walker {
        match result {
            Ok(entry) => {
                let path = entry.path().to_path_buf();
                if path != dir {
                    paths.push(path);
                }
            }
            Err(err) => return Err(format!("Error walking directory: {}", err)),
        }
    }

    Ok(paths)
}

fn format_file_list(paths: &[PathBuf], base: &Path) -> Vec<String> {
    let mut formatted = vec![];

    for (i, path) in paths.iter().enumerate() {
        let rel = path.strip_prefix(base).unwrap_or(path);
        let depth = rel.components().count().saturating_sub(1);
        let name = path.file_name().unwrap_or_default().to_string_lossy();

        let mut line = String::new();
        if depth > 0 {
            line.push_str(&"│   ".repeat(depth - 1));
            let is_last = paths
                .get(i + 1)
                .map(|next| {
                    let next_rel = next.strip_prefix(base).unwrap_or(next);
                    next_rel.components().count().saturating_sub(1) < depth
                })
                .unwrap_or(true);
            line.push_str(if is_last { "└── " } else { "├── " });
        }

        line.push_str(&name);
        if path.is_dir() {
            line.push('/');
        }

        formatted.push(line);
    }

    formatted
}
