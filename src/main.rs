#![feature(nll)]

extern crate getopts;

use getopts::Options;

use std::collections::*;
use std::env;
use std::fs::*;
use std::io;
use std::path::{Path, PathBuf};
use std::process::exit;

/// Print our usage string and exit the program with the given code.
fn print_usage(opts: &Options, code: i32) -> ! {
    if code == 0 {
        println!("{}", opts.usage("Usage: up1 [options] <directory>"));
    } else {
        eprintln!("{}", opts.usage("Usage: up1 [options] <directory>"));
    }
    exit(code);
}

/// Returns true if the given directory contains a single entry.
fn has_one_entry(parent: &Path) -> io::Result<bool> {
    let mut target_iter = read_dir(&parent)?;

    target_iter.next().unwrap_or_else(|| {
        // Invariant: The parent contains at least one entry.
        // (It should at least contain the child).
        panic!(
            "The parent directory ({}) contains no child entries",
            parent.display()
        );
    })?;

    if target_iter.next().is_some() {
        Ok(false)
    } else {
        Ok(true)
    }
}

/// Given some directory, return a unique path in its parent's directory.
/// For example, given foo/bar, return foo/_bar
fn get_unique_temporary_name(parent_dir: &Path) -> io::Result<PathBuf> {
    // We're replacing parent_dir with child_dir, so find _its_ parent
    let grandparent = parent_dir.parent().unwrap_or_else(|| {
        panic!("{} has no parent", parent_dir.display());
    });

    // Get the names of all the items in the grandparent.
    let dir_contents: io::Result<HashSet<PathBuf>> = read_dir(grandparent)?
        .map(|pr| pr.map(|p| p.path()))
        .collect();

    let dir_contents = dir_contents?;

    // Silly (but simple) approach:
    // Put an increasing number of underscores on the parent_dir's name
    // until we get a unique name.
    for num_underscores in 1usize.. {
        let mut with_underscores: String = (0..num_underscores).map(|_| "_").collect();
        with_underscores += parent_dir.file_name().unwrap().to_str().unwrap();

        let mut potentially_unique_name = parent_dir.to_owned();
        potentially_unique_name.set_file_name(with_underscores);

        if !dir_contents.contains(&potentially_unique_name) {
            return Ok(potentially_unique_name);
        }
    }
    unreachable!();
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut opts = Options::new();
    opts.optflag("h", "help", "Print this help text.");
    opts.optflag("n", "dry-run", "Show the commands which would be run.");
    opts.optflag("v", "verbose", "Print each step of the move as it is done.");

    let matches = opts.parse(&args[1..]).unwrap_or_else(|e| {
        // If the user messes up the args, print the error and usage string.
        eprintln!("{}", e.to_string());
        print_usage(&opts, 1);
    });

    if matches.opt_present("h") {
        // Print help as-desired.
        print_usage(&opts, 0);
    }

    if matches.free.len() != 1 {
        eprintln!("Please provide a directory to move up.");
        print_usage(&opts, 1);
    }

    let target_dir = PathBuf::from(&matches.free[0]);

    // Cannonicalize these to make life a touch more straightforward:
    let target_dir = canonicalize(&target_dir).unwrap_or_else(|e| {
        eprintln!(
            "Couldn't canonicalize {}:",
            target_dir.display()
        );
        eprintln!("\t{}", e.to_string());
        exit(1);
    });
    let parent_dir = target_dir.parent().unwrap_or_else(|| {
        eprintln!("{} has no parent.", target_dir.display());
        exit(1);
    });

    if !has_one_entry(parent_dir).unwrap() {
        eprintln!("Can't move the contents of {} up:", target_dir.display());
        eprintln!(
            "{} has more entries than just {}",
            parent_dir.display(),
            target_dir.display()
        );
        exit(1);
    }

    let dry_run = matches.opt_present("n");

    if dry_run {
        println!("Moving the contents of {}", target_dir.display());
        println!("to: {}", parent_dir.display());
        exit(0);
    }

    // Say we want to move the contents of ./foo/bar/ into ./foo/.
    // One way would be to copy or move ./foo/bar/* into ./foo/,
    // but this would turn into lots of I/O operations.
    // Instead, let's
    // 1. Move ./foo/bar to a temporary directory in ./
    // 2. Remove the now-empty ./foo
    // 3. Rename the temp directory to ./foo
    let temp_dir = get_unique_temporary_name(parent_dir).unwrap_or_else(|e| {
        eprintln!(
            "Couldn't create a temporary directory to move {} into:",
            target_dir.display()
        );
        eprintln!("\t{}", e.to_string());
        exit(1);
    });

    let verbose = matches.opt_present("v");

    if verbose {
        eprintln!("Moving {} to {}", target_dir.display(), temp_dir.display());
    }
    rename(&target_dir, &temp_dir).unwrap_or_else(|e| {
        eprintln!(
            "Couldn't move {} to {}:",
            target_dir.display(),
            temp_dir.display()
        );
        eprintln!("\t{}", e.to_string());
        exit(1);
    });

    if verbose {
        eprintln!("Removing the now-empty {}", parent_dir.display());
    }
    remove_dir(&parent_dir).unwrap_or_else(|e| {
        eprintln!("Couldn't remove {}:", parent_dir.display());
        eprintln!("\t{}", e.to_string());
        eprintln!(
            "Note: {} was renamed to {} before this failure.",
            target_dir.display(),
            temp_dir.display()
        );
        exit(1);
    });

    if verbose {
        eprintln!("Moving {} to {}", temp_dir.display(), parent_dir.display());
    }
    rename(&temp_dir, &parent_dir).unwrap_or_else(|e| {
        eprintln!(
            "Couldn't rename {} to {} after removing the original:",
            temp_dir.display(),
            parent_dir.display()
        );
        eprintln!("\t{}", e.to_string());
        exit(1);
    });
}
